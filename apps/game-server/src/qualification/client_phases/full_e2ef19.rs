//! E2EF19: private messages and transient private chat channels between real players.
use super::*;
use oteryn_dev_client::{
    ChatDisposition, ChatIntent, ChatLine, DevClientSession, SessionEvent,
};
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
            evidence(&format!("E2EF19_CHECK S04b {action} PASS observed-native-reply-and-delta"));
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
            result
                .as_ref()
                .err()
                .map_or("private-message-and-channel-lifecycle-observed".into(), |error| {
                    format!("step-reply:{error}").replace(' ', "-")
                }),
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
    let entities = client.world_entities().ok_or("names:world-entities-missing")?;
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
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
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
    for attempt in 0..=2 {
        let reply = client.chat(&intent).await?;
        if reply.disposition == expected {
            return Ok(());
        }
        if !matches!(
            reply.disposition,
            ChatDisposition::Muted | ChatDisposition::Exhausted
        ) || attempt == 2
        {
            return Err(format!("{step}:disposition={:?}", reply.disposition).into());
        }
        client
            .service_liveness(Duration::from_secs(u64::from(reply.wait_seconds.max(1))))
            .await?;
    }
    Err(format!("{step}:retry-bound-exhausted").into())
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

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let mut a = connect_character(clients, 0xe1, 0).await?;
    let mut b = connect_character(clients, 0xe2, 1).await?;
    let result: TestResult = async {
        let a_name = own_name(&a)?;
        let b_name = own_name(&b)?;
        if a_name == b_name {
            return Err("names:two-admissions-have-same-character-name".into());
        }

        accepted(
            &mut a,
            ChatIntent::Private { recipient_name: b_name.clone(), text: "e2ef19-a-pm".into() },
            "private-a-to-b",
        ).await?;
        settle(&mut b).await?;
        if !take_private(&mut b, &a_name, "e2ef19-a-pm") {
            return Err("private-a-to-b:accepted-but-recipient-line-missing".into());
        }
        row.pass("private-a-to-b");

        accepted(
            &mut b,
            ChatIntent::Private { recipient_name: a_name.clone(), text: "e2ef19-b-pm".into() },
            "private-b-to-a",
        ).await?;
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

        accepted(&mut a, ChatIntent::PrivateChannel(PrivateChannelIntent::Create), "channel-create").await?;
        let channel_id = channel_list(&mut a, "channel-create-list").await?.own_channel
            .ok_or("channel-create:accepted-but-own-channel-absent")?;
        row.pass("channel-create");
        accepted(&mut a, ChatIntent::PrivateChannel(PrivateChannelIntent::Invite { name: b_name.clone() }), "channel-invite").await?;
        let invited = channel_list(&mut b, "channel-invite-list").await?;
        if !invited.invited.iter().any(|invite| invite.channel_id == channel_id && invite.owner_name == a_name) {
            return Err("channel-invite:accepted-but-peer-invitation-delta-missing".into());
        }
        row.pass("channel-invite");
        accepted(&mut b, ChatIntent::PrivateChannel(PrivateChannelIntent::Join { channel_id }), "channel-join").await?;
        row.pass("channel-join");

        accepted(&mut a, ChatIntent::PrivateChannel(PrivateChannelIntent::Say { channel_id, text: "e2ef19-a-channel".into() }), "channel-a-to-b").await?;
        settle(&mut b).await?;
        if !take_channel(&mut b, channel_id, &a_name, "e2ef19-a-channel") { return Err("channel-a-to-b:accepted-but-line-missing".into()); }
        row.pass("channel-a-to-b");
        accepted(&mut b, ChatIntent::PrivateChannel(PrivateChannelIntent::Say { channel_id, text: "e2ef19-b-channel".into() }), "channel-b-to-a").await?;
        settle(&mut a).await?;
        if !take_channel(&mut a, channel_id, &b_name, "e2ef19-b-channel") { return Err("channel-b-to-a:accepted-but-line-missing".into()); }
        row.pass("channel-b-to-a");

        accepted(&mut a, ChatIntent::PrivateChannel(PrivateChannelIntent::Exclude { name: b_name.clone() }), "channel-exclude").await?;
        if channel_list(&mut b, "channel-exclude-list").await?.invited.iter().any(|invite| invite.channel_id == channel_id) {
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
        row.pass("excluded-post-refusal");
        accepted(&mut a, ChatIntent::PrivateChannel(PrivateChannelIntent::Leave { channel_id }), "channel-close").await?;
        if channel_list(&mut a, "channel-close-list").await?.own_channel.is_some() { return Err("channel-close:own-channel-still-listed".into()); }
        row.pass("channel-close");

        logout(&mut a).await?;
        a = connect_character(clients, 0xe3, 0).await?;
        if channel_list(&mut a, "relog-transient-list").await?.own_channel.is_some() { return Err("relog-transient:channel-replayed".into()); }
        row.pass("relog-transient");
        accepted(&mut a, ChatIntent::Private { recipient_name: b_name, text: "e2ef19-relog-pm".into() }, "relog-private-message").await?;
        settle(&mut b).await?;
        if !take_private(&mut b, &a_name, "e2ef19-relog-pm") { return Err("relog-private-message:accepted-but-line-missing".into()); }
        row.pass("relog-private-message");
        Ok(())
    }.await;
    let a_logout = logout(&mut a).await;
    let b_logout = logout(&mut b).await;
    result.and(a_logout).and(b_logout)
}
