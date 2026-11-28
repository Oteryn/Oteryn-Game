//! E2EF19: private messages and transient private chat channels between real players.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::chat::private_channel::{ChatChannelList, PrivateChannelIntent};

const ACTIONS: &[&str] = &[
    "private-a-to-b",
    "private-b-to-a",
    "offline-refusal",
    "channel-create",
    "channel-invite",
    "channel-join",
    "channel-a-to-b",
    "channel-b-to-a",
    "channel-exclude",
    "excluded-post-refusal",
    "channel-close",
    "relog-transient",
    "relog-private-message",
];

struct Row {
    passed: std::collections::BTreeSet<&'static str>,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: std::collections::BTreeSet::new(),
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, action: &'static str) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!(
                "E2EF19_CHECK S04b {action} PASS observed-native-reply-and-delta"
            ));
        }
    }

    fn finish(self, result: &TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("E2EF19_GAP S04b {action} unproven"));
            }
        }
        if let Err(error) = result {
            evidence(&format!("E2EF19_GAP S04b observed={error}"));
        }
        evidence(&format!(
            "e2e_all scenario=S04b.full status={} seconds={:.3} evidence={}",
            if result.is_ok() && self.passed.len() == ACTIONS.len() {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
            result.as_ref().err().map_or(
                "private-message-and-channel-lifecycle-observed".into(),
                |error| { format!("step-reply:{error}").replace(' ', "-") }
            ),
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = tokio::time::timeout(Duration::from_secs(180), journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| Err("journey-timeout:no-further-reply-frame".into()));
    row.finish(&result);
}

fn own_name(client: &DevClientSession) -> TestResult<String> {
    let entities = client
        .world_entities()
        .ok_or("names:world-entities-missing")?;
    let name = entities
        .iter()
        .find(|entity| entity.entity.identity == *entities.own_identity())
        .and_then(|entity| entity.display_name.as_ref())
        .ok_or("names:own-display-name-missing")?;
    Ok(name
        .as_str()
        .map_err(|error| format!("names:invalid-display-name:{error:?}"))?
        .to_owned())
}

async fn settle(client: &mut DevClientSession) -> TestResult {
    client.service_liveness(Duration::from_millis(250)).await?;
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let session = client.join_snapshot().game_session_id;
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        evidence(&format!(
            "S04B_LOGOUT session={} status={:?} result={:?}",
            uuid_text(session.as_bytes()),
            reply.status,
            reply.result
        ));
        return Err(format!("logout:status={:?}:result={:?}", reply.status, reply.result).into());
    }
    Ok(())
}

async fn accepted(client: &mut DevClientSession, intent: ChatIntent, step: &str) -> TestResult {
    expect_disposition(client, intent, ChatDisposition::Ok, step).await
}

async fn expect_disposition(
    client: &mut DevClientSession,
    intent: ChatIntent,
    expected: ChatDisposition,
    step: &str,
) -> TestResult {
    let reply = client.chat(&intent).await?;
    if reply.disposition != expected {
        return Err(format!("{step}:disposition={:?}", reply.disposition).into());
    }
    Ok(())
}

fn take_private(client: &mut DevClientSession, speaker: &str, text: &str) -> bool {
    client.take_events().into_iter().any(|event| {
        matches!(event, SessionEvent::ChatLine(delta)
            if matches!(&delta.value, ChatLine::Private { speaker_name, text: line }
                if speaker_name == speaker && line == text))
    })
}

fn take_channel(client: &mut DevClientSession, channel_id: u16, speaker: &str, text: &str) -> bool {
    client.take_events().into_iter().any(|event| {
        matches!(event, SessionEvent::ChatLine(delta)
            if matches!(&delta.value, ChatLine::PrivateChannel(line)
                if line.channel_id == channel_id && line.speaker_name == speaker && line.text == text))
    })
}

async fn channel_list(client: &mut DevClientSession, step: &str) -> TestResult<ChatChannelList> {
    accepted(
        client,
        ChatIntent::PrivateChannel(PrivateChannelIntent::List),
        step,
    )
    .await?;
    settle(client).await?;
    client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::ChatChannelList(delta) => Some(delta.value),
            _ => None,
        })
        .next_back()
        .ok_or_else(|| format!("{step}:accepted-but-channel-list-delta-missing").into())
}

async fn prepare_sender(
    clients: &SeamClients<'_>,
    tag: u8,
    role: &str,
) -> TestResult<DevClientSession> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;

    let mut client = connect_character(clients, tag, 0).await?;
    let session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(session.as_bytes()),
        "--profile",
        "s04b-level".into(),
    )
    .await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, tag, 0).await?;
    if client.join_snapshot().game_session_id == session {
        return Err("S04b level fixture reused the original admission".into());
    }
    let stats = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[wire::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if stats.status != CommandStatus::Accepted {
        return Err(format!("S04b level read refused for {role}: {:?}", stats.status).into());
    }
    general_stats::validate(&stats.payload, general_stats::selected(&client))?;
    let level = u32::from_le_bytes(stats.payload[9..13].try_into()?);
    if level != 3 {
        return Err(format!("S04b seeded level for {role}: expected 3, got {level}").into());
    }
    evidence(&format!(
        "S04B_FIXTURE account={role} level={level} verified=after_relog"
    ));
    Ok(client)
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let owner = character_prep::dedicated_character(clients, "NODE_BOOT_Q_S04B_ACCOUNT").await?;
    let peer =
        character_prep::dedicated_character(clients, "NODE_BOOT_Q_S04B_PEER_ACCOUNT").await?;
    let owner_clients = owner.clients(clients);
    let peer_clients = peer.clients(clients);
    character_prep::declare_seeded(
        "S04b.full",
        "two level-3 senders; Premium channel owner",
        "Private messaging requires level 3; leveling and entitlement acquisition are outside this journey",
    );
    let mut a = prepare_sender(&owner_clients, 0xe1, "q_s04b").await?;
    let mut b =
        full_e2ef4::keep_alive(&mut a, prepare_sender(&peer_clients, 0xe2, "q_s04b_peer")).await?;
    let result: TestResult = async {
        let a_name = own_name(&a)?;
        let b_name = own_name(&b)?;
        if a_name == b_name {
            return Err("names:two-admissions-have-same-character-name".into());
        }

        let sender_session = a.join_snapshot().game_session_id;
        let recipient_session = b.join_snapshot().game_session_id;
        let reply = mp::observations::while_servicing_liveness(
            a.chat(&ChatIntent::Private {
                recipient_name: b_name.clone(),
                text: "e2ef19-a-pm".into(),
            }),
            async || {
                b.service_liveness(Duration::from_millis(50)).await?;
                Ok(())
            },
        )
        .await??;
        evidence(&format!(
            "S04B_PRIVATE step=private-a-to-b sender={a_name:?} sender_session={} recipient={b_name:?} recipient_id={} recipient_session={} world={} path=world_relay command_id={} server_sequence={} disposition={:?}",
            uuid_text(sender_session.as_bytes()),
            uuid_text(&peer_clients.characters[0]),
            uuid_text(recipient_session.as_bytes()),
            uuid_text(&clients.world),
            reply.command_id,
            reply.result_server_sequence,
            reply.disposition,
        ));
        if reply.disposition != ChatDisposition::Ok {
            return Err(format!("private-a-to-b:disposition={:?}", reply.disposition).into());
        }
        settle(&mut b).await?;
        if !take_private(&mut b, &a_name, "e2ef19-a-pm") {
            return Err("private-a-to-b:accepted-but-recipient-line-missing".into());
        }
        row.pass("private-a-to-b");

        accepted(
            &mut b,
            ChatIntent::Private {
                recipient_name: a_name.clone(),
                text: "e2ef19-b-pm".into(),
            },
            "private-b-to-a",
        )
        .await?;
        settle(&mut a).await?;
        if !take_private(&mut a, &b_name, "e2ef19-b-pm") {
            return Err("private-b-to-a:accepted-but-recipient-line-missing".into());
        }
        row.pass("private-b-to-a");

        expect_disposition(
            &mut a,
            ChatIntent::Private {
                recipient_name: "E2ef19 Nobody".into(),
                text: "not delivered".into(),
            },
            ChatDisposition::NotOnline,
            "offline-refusal",
        )
        .await?;
        row.pass("offline-refusal");

        character_prep::grant_premium(&a).await?;
        evidence("S04B_FIXTURE account=q_s04b premium=granted role=channel_owner");
        if channel_list(&mut a, "channel-create-pre-list")
            .await?
            .own_channel
            .is_some()
        {
            return Err("channel-create:own-channel-already-listed".into());
        }
        accepted(
            &mut a,
            ChatIntent::PrivateChannel(PrivateChannelIntent::Create),
            "channel-create",
        )
        .await?;
        let channel_id = channel_list(&mut a, "channel-create-list")
            .await?
            .own_channel
            .ok_or("channel-create:accepted-but-own-channel-absent")?;
        row.pass("channel-create");
        accepted(
            &mut a,
            ChatIntent::PrivateChannel(PrivateChannelIntent::Invite {
                name: b_name.clone(),
            }),
            "channel-invite",
        )
        .await?;
        let invited = channel_list(&mut b, "channel-invite-list").await?;
        if !invited
            .invited
            .iter()
            .any(|invite| invite.channel_id == channel_id && invite.owner_name == a_name)
        {
            return Err("channel-invite:accepted-but-peer-invitation-delta-missing".into());
        }
        row.pass("channel-invite");
        accepted(
            &mut b,
            ChatIntent::PrivateChannel(PrivateChannelIntent::Join { channel_id }),
            "channel-join",
        )
        .await?;
        row.pass("channel-join");

        accepted(
            &mut a,
            ChatIntent::PrivateChannel(PrivateChannelIntent::Say {
                channel_id,
                text: "e2ef19-a-channel".into(),
            }),
            "channel-a-to-b",
        )
        .await?;
        settle(&mut b).await?;
        if !take_channel(&mut b, channel_id, &a_name, "e2ef19-a-channel") {
            return Err("channel-a-to-b:accepted-but-line-missing".into());
        }
        row.pass("channel-a-to-b");
        accepted(
            &mut b,
            ChatIntent::PrivateChannel(PrivateChannelIntent::Say {
                channel_id,
                text: "e2ef19-b-channel".into(),
            }),
            "channel-b-to-a",
        )
        .await?;
        settle(&mut a).await?;
        if !take_channel(&mut a, channel_id, &b_name, "e2ef19-b-channel") {
            return Err("channel-b-to-a:accepted-but-line-missing".into());
        }
        row.pass("channel-b-to-a");

        accepted(
            &mut a,
            ChatIntent::PrivateChannel(PrivateChannelIntent::Exclude {
                name: b_name.clone(),
            }),
            "channel-exclude",
        )
        .await?;
        let before_refusal = channel_list(&mut b, "channel-exclude-list").await?;
        if before_refusal
            .invited
            .iter()
            .any(|invite| invite.channel_id == channel_id)
        {
            return Err("channel-exclude:peer-still-invited".into());
        }
        row.pass("channel-exclude");
        expect_disposition(
            &mut b,
            ChatIntent::PrivateChannel(PrivateChannelIntent::Say {
                channel_id,
                text: "must refuse".into(),
            }),
            ChatDisposition::NotInvited,
            "excluded-post-refusal",
        )
        .await?;
        if channel_list(&mut b, "excluded-post-list").await? != before_refusal {
            return Err("excluded-post-refusal:peer-channel-list-changed".into());
        }
        settle(&mut a).await?;
        if take_channel(&mut a, channel_id, &b_name, "must refuse") {
            return Err("excluded-post-refusal:refused-line-delivered".into());
        }
        row.pass("excluded-post-refusal");
        accepted(
            &mut a,
            ChatIntent::PrivateChannel(PrivateChannelIntent::Leave { channel_id }),
            "channel-close",
        )
        .await?;
        if channel_list(&mut a, "channel-close-list")
            .await?
            .own_channel
            .is_some()
        {
            return Err("channel-close:own-channel-still-listed".into());
        }
        row.pass("channel-close");

        accepted(
            &mut a,
            ChatIntent::PrivateChannel(PrivateChannelIntent::Create),
            "relog-create",
        )
        .await?;
        if channel_list(&mut a, "relog-pre-list")
            .await?
            .own_channel
            .is_none()
        {
            return Err("relog-transient:no-open-channel-before-logout".into());
        }
        logout(&mut a).await?;
        a = connect_character(&owner_clients, 0xe3, 0).await?;
        if channel_list(&mut a, "relog-transient-list")
            .await?
            .own_channel
            .is_some()
        {
            return Err("relog-transient:channel-replayed".into());
        }
        row.pass("relog-transient");
        accepted(
            &mut a,
            ChatIntent::Private {
                recipient_name: b_name,
                text: "e2ef19-relog-pm".into(),
            },
            "relog-private-message",
        )
        .await?;
        settle(&mut b).await?;
        if !take_private(&mut b, &a_name, "e2ef19-relog-pm") {
            return Err("relog-private-message:accepted-but-line-missing".into());
        }
        row.pass("relog-private-message");
        Ok(())
    }
    .await;
    let a_logout = logout(&mut a).await;
    let b_logout = logout(&mut b).await;
    result.and(a_logout).and(b_logout)
}
