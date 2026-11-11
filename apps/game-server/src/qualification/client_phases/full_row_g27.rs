//! G27: account friends and badge visibility (caps 43 and 199).
//! Two sessions (the two provisioned accounts): both choose a main character, the first invites
//! the second by its main-character name, the inviter's own accept is refused (code 10), the
//! invitee accepts, the inviter sees the main character, and a revoke empties the list. Badge
//! visibility is then driven through the Cyclopedia badge request; a character with no earned
//! badge cannot toggle one, so that leg ends FAIL `needed:earned-badge`.
use super::*;
use oteryn_dev_client::DevClientSession;
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(180), journey(clients)).await;
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

async fn logout(client: &mut RowClient) -> TestResult {
    if client.row_logout().await?.result
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
    let reply = raw_feature::send(client.session_mut(), &command).await?;
    Ok(cyc::apply_result(panel, &reply))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let (name_a, name_b) = (
        character_name(clients, 0).await?,
        character_name(clients, 1).await?,
    );
    let mut a = row_connect_character(clients, 0xd7, 0).await?;
    let mut b = row_connect_world(clients, 0xd8).await?;
    for client in [&a, &b] {
        for capability in [vip::CAPABILITY_VIP_V1, 199] {
            if !client.selected_capabilities().contains(&capability) {
                return Err(
                    format!("step=admit observed=capability-{capability}-not-selected").into(),
                );
            }
        }
    }
    for client in [&mut a, &mut b] {
        let (code, _) = friends(client, vip::Operation::MainCharacter, 0, "").await?;
        if code != 1 {
            return Err(format!("step=main-character observed=code:{code};needed=1").into());
        }
    }
    let (code, _) = friends(&mut a, vip::Operation::FriendInvite, 0, &name_b).await?;
    if code != 1 {
        return Err(format!("step=invite observed=code:{code};needed=1").into());
    }
    let (_, view) = friends(&mut a, vip::Operation::FriendsQuery, 0, "").await?;
    if view["account_friends"][0]["state"] != "outgoing" {
        return Err(format!(
            "step=outgoing observed={};needed=outgoing",
            view["account_friends"][0]["state"]
        )
        .into());
    }
    let outgoing = view["account_friends"][0]["handle"]
        .as_u64()
        .ok_or("step=outgoing observed=no-handle")?;
    let (self_accept, _) = friends(&mut a, vip::Operation::FriendAccept, outgoing, "").await?;
    if self_accept != 10 {
        return Err(format!("step=self-accept observed=code:{self_accept};needed=10").into());
    }
    let (_, view) = friends(&mut b, vip::Operation::FriendsQuery, 0, "").await?;
    let incoming = view["account_friends"][0]["handle"]
        .as_u64()
        .ok_or("step=incoming observed=no-handle")?;
    let (code, _) = friends(&mut b, vip::Operation::FriendAccept, incoming, "").await?;
    if code != 1 {
        return Err(format!("step=accept observed=code:{code};needed=1").into());
    }
    let (_, view) = friends(&mut a, vip::Operation::FriendsQuery, 0, "").await?;
    if view["account_friends"][0]["main_character"].as_str() != Some(name_b.as_str()) {
        return Err(format!(
            "step=accepted-view observed={};needed={name_b}",
            view["account_friends"][0]["main_character"]
        )
        .into());
    }
    logout(&mut a).await?;
    logout(&mut b).await?;
    a = row_connect_character(clients, 0xa7, 1).await?;
    b = row_connect_character(clients, 0xb7, 0).await?;
    let (_, persisted) = friends(&mut a, vip::Operation::FriendsQuery, 0, "").await?;
    if persisted["account_friends"][0]["main_character"].as_str() != Some(name_b.as_str()) {
        return Err("step=fresh-admission observed=accepted-friendship-lost".into());
    }
    let outgoing = persisted["account_friends"][0]["handle"]
        .as_u64()
        .ok_or("step=fresh-admission observed=friend-handle-absent")?;
    let mut panel = cyc::CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    if !panel_call(&mut a, &mut panel, cyc::Intent::Badges).await? {
        return Err("step=badges observed=reply-not-applied".into());
    }
    let earned = panel
        .badges
        .clone()
        .ok_or("step=badges observed=no-badge-list")?;
    let mut badge_note = "no-earned-badge".to_owned();
    if let Some(&badge) = earned.first() {
        if !panel_call(
            &mut a,
            &mut panel,
            cyc::Intent::SetBadgeVisibility {
                badge,
                public: false,
            },
        )
        .await?
        {
            return Err("step=badge-hide observed=reply-not-applied".into());
        }
        logout(&mut a).await?;
        a = row_connect_character(clients, 0xa7, 1).await?;
        panel = cyc::CyclopediaCharacterPanel {
            selected: true,
            ..Default::default()
        };
        if !panel_call(&mut a, &mut panel, cyc::Intent::Badges).await?
            || !panel.badge_visibility_known
            || !panel.server_hidden_badges.contains(&badge)
        {
            return Err(format!(
                "step=badge-hide observed=server-hidden:{:?};known:{};needed=contains-{badge}",
                panel.server_hidden_badges, panel.badge_visibility_known
            )
            .into());
        }
        if !panel_call(
            &mut a,
            &mut panel,
            cyc::Intent::SetBadgeVisibility {
                badge,
                public: true,
            },
        )
        .await?
        {
            return Err("step=badge-restore observed=reply-not-applied".into());
        }
        logout(&mut a).await?;
        a = row_connect_character(clients, 0xa7, 1).await?;
        panel = cyc::CyclopediaCharacterPanel {
            selected: true,
            ..Default::default()
        };
        if !panel_call(&mut a, &mut panel, cyc::Intent::Badges).await?
            || !panel.badge_visibility_known
            || panel.server_hidden_badges.contains(&badge)
        {
            return Err(format!(
                "step=badge-restore observed=server-hidden:{:?};known:{};needed=not-contains-{badge}",
                panel.server_hidden_badges, panel.badge_visibility_known
            )
            .into());
        }
        badge_note = format!("badge-{badge}-hidden-and-restored");
    }
    let (_, current) = friends(&mut a, vip::Operation::FriendsQuery, 0, "").await?;
    let outgoing = current["account_friends"][0]["handle"]
        .as_u64()
        .unwrap_or(outgoing);
    let (code, _) = friends(&mut a, vip::Operation::FriendRevoke, outgoing, "").await?;
    if code != 1 {
        return Err(format!("step=revoke observed=code:{code};needed=1").into());
    }
    let (_, view) = friends(&mut a, vip::Operation::FriendsQuery, 0, "").await?;
    if !view["account_friends"]
        .as_array()
        .is_some_and(|rows| rows.is_empty())
    {
        return Err("step=revoke observed=friend-still-listed".into());
    }
    let _ = name_a;
    logout(&mut a).await?;
    logout(&mut b).await?;
    a = row_connect_character(clients, 0xa7, 1).await?;
    b = row_connect_character(clients, 0xb7, 0).await?;
    for client in [&mut a, &mut b] {
        let (_, persisted) = friends(client, vip::Operation::FriendsQuery, 0, "").await?;
        if !persisted["account_friends"]
            .as_array()
            .is_some_and(|rows| rows.is_empty())
        {
            return Err("step=fresh-admission observed=revoked-friendship-restored".into());
        }
    }
    logout(&mut a).await?;
    logout(&mut b).await?;
    if earned.is_empty() {
        return Err(format!("friends-invite-accept-revoke-proven;step=badge-visibility observed={badge_note};needed:earned-badge").into());
    }
    Ok(format!("friends-and-{badge_note}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_tokens_cover_empty_and_whitespace() {
        assert_eq!(token(""), "");
        assert!(!token("a b\tc\nd").chars().any(char::is_whitespace));
        assert_eq!(token("source:path.rs:12"), "source:path.rs:12");
    }
}
