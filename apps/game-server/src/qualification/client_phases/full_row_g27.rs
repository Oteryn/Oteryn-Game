//! G27: account friends and badge visibility (caps 43 and 199).
//! Two sessions (the two provisioned accounts): both choose a main character, the first invites
//! the second by its main-character name, the inviter's own accept is refused (code 10), the
//! invitee accepts, the inviter sees the main character, and a revoke empties the list. Badge
//! visibility is driven through the Cyclopedia badge request for Fledgeling Hero.
//! Fresh sessions verify the changed friend state and the stored badge preferences.
use super::*;
use oteryn_dev_client::DevClientSession;
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G27.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn character_name(clients: &SeamClients<'_>, index: usize) -> TestResult<String> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id = encode($1, 'hex')::uuid",
    )
    .bind(clients.characters[index].as_slice())
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    Ok(name)
}

use oteryn_dev_client::client_runtime::raw_feature;
use oteryn_dev_client::client_ui::cyclopedia_character as cyc;
use oteryn_protocol_oteryn::vip;

async fn friends(
    client: &mut DevClientSession,
    operation: vip::Operation,
    number: u64,
    text: &str,
) -> TestResult<(u8, serde_json::Value)> {
    let (kind, bytes) = vip::encode_request(&vip::Request {
        operation,
        number,
        text: text.into(),
        icon: 0,
        notify: false,
        groups: vec![],
    })
    .map_err(|error| format!("step=vip-encode observed={error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(vip::CAPABILITY_VIP_V1, kind, &bytes)
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!(
            "step=vip-{operation:?} observed=status:{:?}",
            response.status
        )
        .into());
    }
    let result = vip::decode_result(&response.payload)
        .map_err(|error| format!("step=vip-decode observed={error:?}"))?;
    let view = if result.view.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(&result.view)?
    };
    Ok((result.code, view))
}

async fn panel_call(
    client: &mut DevClientSession,
    panel: &mut cyc::CyclopediaCharacterPanel,
    intent: cyc::Intent,
) -> TestResult<bool> {
    let command = cyc::request(panel, intent).ok_or_else(|| {
        format!(
            "step=cyclopedia-request observed=local-refusal:{:?}",
            panel.feedback
        )
    })?;
    let reply = client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if let cyc::Intent::SetBadgeVisibility { badge, public } = intent
        && !badge_set_acknowledged(&reply, badge, public)
    {
        return Err(format!(
                "step=badge-set-ack observed=status:{:?},sequence:{},payload:{:?};needed=accepted-visibility-{badge}-{public}",
                reply.status, reply.server_sequence, reply.payload
            ).into());
    }
    Ok(cyc::apply_result(panel, &reply))
}

// Only a fresh, typed acknowledgment with the requested value can prove a setter.
fn badge_set_acknowledged(
    reply: &raw_feature::GatedCommandResult,
    badge: u8,
    public: bool,
) -> bool {
    use oteryn_protocol_oteryn::cyclopedia_character::badge_visibility::Reply;
    reply.status == CommandStatus::Accepted
        && Reply::decode(&reply.payload).is_ok_and(|state| {
            state.earned.iter().any(|earned| earned.id() == badge)
                && state.hidden.iter().any(|hidden| hidden.id() == badge) != public
        })
}

#[cfg(test)]
mod badge_ack_tests {
    use super::*;
    use oteryn_protocol_oteryn::cyclopedia_character::{Badge, badge_visibility::Reply};

    #[test]
    fn setter_requires_fresh_typed_acknowledgment_of_requested_visibility() -> TestResult {
        let badge = Badge::FledgelingHero;
        let mut reply = raw_feature::GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: Reply {
                earned: vec![badge],
                hidden: vec![badge],
            }
            .encode()
            .map_err(|_| "badge acknowledgment test encoding failed")?,
        };
        assert!(badge_set_acknowledged(&reply, badge.id(), false));
        assert!(!badge_set_acknowledged(&reply, badge.id(), true));
        reply.status = CommandStatus::DuplicateReplay;
        assert!(!badge_set_acknowledged(&reply, badge.id(), false));
        reply.status = CommandStatus::Accepted;
        reply.payload = vec![1, badge.id()];
        assert!(!badge_set_acknowledged(&reply, badge.id(), true));
        reply.payload = Reply {
            earned: vec![],
            hidden: vec![],
        }
        .encode()
        .map_err(|_| "badge acknowledgment test encoding failed")?;
        assert!(!badge_set_acknowledged(&reply, badge.id(), true));
        reply.payload = Reply {
            earned: vec![badge],
            hidden: vec![],
        }
        .encode()
        .map_err(|_| "badge acknowledgment test encoding failed")?;
        assert!(badge_set_acknowledged(&reply, badge.id(), true));
        Ok(())
    }
}

async fn expect_friends(
    client: &mut DevClientSession,
    step: &str,
    main: Option<&str>,
    peer: Option<(&str, &str, Option<&str>)>,
) -> TestResult<Option<u64>> {
    let (code, view) = friends(client, vip::Operation::FriendsQuery, 0, "").await?;
    let handle = view["account_friends"][0]["handle"].as_u64();
    let expected = match peer {
        Some((name, state, peer_main)) => {
            let handle = handle
                .filter(|handle| *handle != 0)
                .ok_or_else(|| format!("step={step} observed=missing-or-invalid-friend-handle"))?;
            serde_json::json!([{
                "handle": handle, "name": name, "state": state, "main_character": peer_main,
            }])
        }
        None => serde_json::json!([]),
    };
    let expected_main = serde_json::json!(main);
    if code != 1
        || view.get("main_character") != Some(&expected_main)
        || view["next_friend"] != 0
        || view["account_friends"] != expected
    {
        return Err(format!("step={step} observed=code:{code},view:{view};needed=main:{expected_main},friends:{expected}").into());
    }
    Ok(handle)
}

async fn relog(
    clients: &SeamClients<'_>,
    mut client: DevClientSession,
    tag: u8,
    index: usize,
) -> TestResult<DevClientSession> {
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let fresh = connect_character(clients, tag, index).await?;
    if fresh.join_snapshot().game_session_id == session {
        return Err(format!("step=relog account-index={index} observed=reused-session").into());
    }
    Ok(fresh)
}

async fn expect_badges(
    client: &mut DevClientSession,
    step: &str,
    hidden: &[u8],
) -> TestResult<cyc::CyclopediaCharacterPanel> {
    let mut panel = cyc::CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    let badge = oteryn_protocol_oteryn::cyclopedia_character::Badge::FledgelingHero as u8;
    if !panel_call(client, &mut panel, cyc::Intent::Badges).await?
        || !panel.badge_visibility_known
        || !panel
            .badges
            .as_ref()
            .is_some_and(|earned| earned.contains(&badge))
        || !panel
            .server_hidden_badges
            .iter()
            .copied()
            .eq(hidden.iter().copied())
    {
        return Err(format!(
            "step={step} observed=earned:{:?},hidden:{:?},known:{};needed=earned-{badge},hidden:{hidden:?}",
            panel.badges, panel.server_hidden_badges, panel.badge_visibility_known
        )
        .into());
    }
    Ok(panel)
}

// The wire view defaults to public when there is no stored preference.
async fn expect_peer_profile_badge(
    peer: &mut DevClientSession,
    target: [u8; 16],
    name: &str,
    badge: u8,
    public: bool,
) -> TestResult {
    use oteryn_protocol_oteryn::cyclopedia_character::{
        CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, profile,
    };
    let reply = peer
        .session_mut()
        .gated_command(
            CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &profile::request(Some(target)),
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=peer-badge-profile observed=status:{:?}", reply.status).into());
    }
    let view = profile::Profile::decode(&reply.payload)
        .map_err(|error| format!("step=peer-badge-profile observed=decode:{error:?}"))?;
    let visible = view.badges.iter().any(|earned| earned.id() == badge);
    if view.character != target || view.name != name || visible != public {
        return Err(format!(
            "step=peer-badge-profile observed=name:{},badge-{badge}-public:{visible};needed=name:{name},badge-{badge}-public:{public}",
            view.name
        ).into());
    }
    Ok(())
}

async fn expect_badge_preference(clients: &SeamClients<'_>, badge: u8, public: bool) -> TestResult {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let stored: Option<bool> = sqlx::query_scalar(
        "SELECT v.is_public FROM game_account_badge_visibility v
         JOIN game_character_roots c USING (account_id)
         WHERE c.character_id = encode($1, 'hex')::uuid AND v.badge = $2",
    )
    .bind(clients.characters[0].as_slice())
    .bind(i16::from(badge))
    .fetch_optional(&mut database)
    .await?;
    database.close().await?;
    if stored != Some(public) {
        return Err(format!(
            "step=badge-preference-after-relog observed={stored:?};needed={public}"
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let (name_a, name_b) = (
        character_name(clients, 0).await?,
        character_name(clients, 1).await?,
    );
    let mut a = connect_character(clients, 0xd7, 0).await?;
    let mut b = connect_world(clients, 0xd8).await?;
    for client in [&a, &b] {
        for capability in [vip::CAPABILITY_VIP_V1, 199] {
            if !client.selected_capabilities().contains(&capability) {
                return Err(
                    format!("step=admit observed=capability-{capability}-not-selected").into(),
                );
            }
        }
    }
    for (client, name) in [(&mut a, name_a.as_str()), (&mut b, name_b.as_str())] {
        expect_friends(client, "social-pre-state", None, None).await?;
        let (code, _) = friends(client, vip::Operation::MainCharacter, 0, "").await?;
        if code != 1 {
            return Err(format!("step=main-character observed=code:{code};needed=1").into());
        }
        expect_friends(client, "main-character", Some(name), None).await?;
    }
    let (code, _) = friends(&mut a, vip::Operation::FriendInvite, 0, &name_b).await?;
    if code != 1 {
        return Err(format!("step=invite observed=code:{code};needed=1").into());
    }
    let outgoing = Some((name_b.as_str(), "outgoing", None));
    let incoming = Some((name_a.as_str(), "incoming", None));
    let a_main = Some(name_a.as_str());
    let b_main = Some(name_b.as_str());
    expect_friends(&mut a, "outgoing", a_main, outgoing).await?;
    expect_friends(&mut b, "incoming", b_main, incoming).await?;
    a = relog(clients, a, 0xd7, 0).await?;
    b = relog(clients, b, 0xd8, 1).await?;
    let outgoing_handle = expect_friends(&mut a, "outgoing-after-relog", a_main, outgoing)
        .await?
        .ok_or("step=outgoing-after-relog observed=no-handle")?;
    let incoming_handle = expect_friends(&mut b, "incoming-after-relog", b_main, incoming)
        .await?
        .ok_or("step=incoming-after-relog observed=no-handle")?;
    let (self_accept, _) =
        friends(&mut a, vip::Operation::FriendAccept, outgoing_handle, "").await?;
    if self_accept != 10 {
        return Err(format!("step=self-accept observed=code:{self_accept};needed=10").into());
    }
    let a_after =
        expect_friends(&mut a, "self-accept-unchanged-outgoing", a_main, outgoing).await?;
    let b_after =
        expect_friends(&mut b, "self-accept-unchanged-incoming", b_main, incoming).await?;
    if a_after != Some(outgoing_handle) || b_after != Some(incoming_handle) {
        return Err("step=self-accept observed=handles-changed".into());
    }
    let (code, _) = friends(&mut b, vip::Operation::FriendAccept, incoming_handle, "").await?;
    if code != 1 {
        return Err(format!("step=accept observed=code:{code};needed=1").into());
    }
    let a_friend = Some((name_b.as_str(), "friend", b_main));
    let b_friend = Some((name_a.as_str(), "friend", a_main));
    expect_friends(&mut a, "accepted-view-inviter", a_main, a_friend).await?;
    expect_friends(&mut b, "accepted-view-invitee", b_main, b_friend).await?;

    // Profile access uses character VIP consent, independently of account friendship.
    let (add, _) = friends(&mut b, vip::Operation::Add, 0, &name_a).await?;
    let (query, view) = friends(&mut a, vip::Operation::Query, 0, "").await?;
    let invitation = &view["consents"][0];
    if add != 1 || query != 1 || invitation["name"].as_str() != Some(name_b.as_str()) {
        return Err(format!(
            "step=profile-consent-invite observed=add:{add},query:{query},invite:{invitation}"
        )
        .into());
    }
    let handle = invitation["handle"]
        .as_u64()
        .ok_or("step=profile-consent-invite observed=no-handle")?;
    let (accept, _) = friends(&mut a, vip::Operation::Accept, handle, "").await?;
    if accept != 1 {
        return Err(format!("step=profile-consent-accept observed=code:{accept};needed=1").into());
    }

    let badge = oteryn_protocol_oteryn::cyclopedia_character::Badge::FledgelingHero as u8;
    expect_peer_profile_badge(&mut b, clients.characters[0], &name_a, badge, true).await?;
    for (public, before, after, step) in [
        (false, &[][..], &[badge][..], "badge-hide"),
        (true, &[badge][..], &[][..], "badge-restore"),
    ] {
        let mut panel = expect_badges(&mut a, step, before).await?;
        if !panel_call(
            &mut a,
            &mut panel,
            cyc::Intent::SetBadgeVisibility { badge, public },
        )
        .await?
        {
            return Err(format!("step={step} observed=reply-not-applied").into());
        }
        expect_badges(&mut a, step, after).await?;
        a = relog(clients, a, 0xd7, 0).await?;
        expect_badges(&mut a, &format!("{step}-after-relog"), after).await?;
        expect_badge_preference(clients, badge, public).await?;
        expect_peer_profile_badge(&mut b, clients.characters[0], &name_a, badge, public).await?;
    }
    b = relog(clients, b, 0xd8, 1).await?;
    let accepted_handle = expect_friends(&mut a, "accepted-after-relog-inviter", a_main, a_friend)
        .await?
        .ok_or("step=accepted-after-relog-inviter observed=no-handle")?;
    expect_friends(&mut b, "accepted-after-relog-invitee", b_main, b_friend).await?;

    let (code, _) = friends(&mut a, vip::Operation::FriendRevoke, accepted_handle, "").await?;
    if code != 1 {
        return Err(format!("step=revoke observed=code:{code};needed=1").into());
    }
    expect_friends(&mut a, "revoked-inviter", a_main, None).await?;
    expect_friends(&mut b, "revoked-invitee", b_main, None).await?;
    a = relog(clients, a, 0xd7, 0).await?;
    b = relog(clients, b, 0xd8, 1).await?;
    expect_friends(&mut a, "revoked-after-relog-inviter", a_main, None).await?;
    expect_friends(&mut b, "revoked-after-relog-invitee", b_main, None).await?;
    logout(&mut a).await?;
    logout(&mut b).await?;
    Ok(format!(
        "friends-and-badge-{badge}-hidden-and-restored-after-relog"
    ))
}
