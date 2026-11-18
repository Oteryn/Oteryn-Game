//! G27: account friends and badge visibility (caps 43 and 199).
//! Two sessions (the two provisioned accounts): both choose a main character, the first invites
//! the second by its main-character name, the inviter's own accept is refused (code 10), the
//! invitee accepts, the inviter sees the main character, and a revoke removes that friend from both lists. Badge
//! visibility is then driven through the Cyclopedia badge request; a character with no earned
//! badge cannot toggle one, so that leg ends FAIL `needed:earned-badge`.
use super::*;
use oteryn_dev_client::DevClientSession;
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients))).await;
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

fn peer_row<'a>(rows: &'a [serde_json::Value], peer: &str) -> Option<&'a serde_json::Value> {
    rows.iter().find(|row| row["name"].as_str() == Some(peer))
}

async fn find_peer(client: &mut DevClientSession, name: &str) -> TestResult<Option<serde_json::Value>> {
    let mut after = 0;
    loop {
        let (code, view) = friends(client, vip::Operation::FriendsQuery, after, "").await?;
        let rows = view["account_friends"].as_array().filter(|_| code == 1)
            .ok_or("step=friends-query observed=missing-list-or-refusal")?;
        if let Some(row) = peer_row(rows, name) {
            return Ok(Some(row.clone()));
        }
        let next = view["next_friend"].as_u64().ok_or("step=friends-query observed=missing-cursor")?;
        if next == 0 {
            return Ok(None);
        }
        if next <= after {
            return Err(format!("step=friends-query observed=non-advancing-cursor:{next}").into());
        }
        after = next;
    }
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
    let row = find_peer(&mut a, &name_b).await?.ok_or("step=outgoing observed=peer-not-listed")?;
    if row["state"] != "outgoing" {
        return Err(format!("step=outgoing observed={row};needed=outgoing").into());
    }
    let outgoing = row["handle"].as_u64().ok_or("step=outgoing observed=no-handle")?;
    let (self_accept, _) = friends(&mut a, vip::Operation::FriendAccept, outgoing, "").await?;
    if self_accept != 10 {
        return Err(format!("step=self-accept observed=code:{self_accept};needed=10").into());
    }
    let row = find_peer(&mut b, &name_a).await?.ok_or("step=incoming observed=peer-not-listed")?;
    if row["state"] != "incoming" {
        return Err(format!("step=incoming observed={row};needed=incoming").into());
    }
    let incoming = row["handle"].as_u64().ok_or("step=incoming observed=no-handle")?;
    let (code, _) = friends(&mut b, vip::Operation::FriendAccept, incoming, "").await?;
    if code != 1 {
        return Err(format!("step=accept observed=code:{code};needed=1").into());
    }
    for (client, peer) in [(&mut a, &name_b), (&mut b, &name_a)] {
        let row = find_peer(client, peer).await?.ok_or("step=accepted-view observed=peer-not-listed")?;
        if row["state"] != "friend" || row["main_character"].as_str() != Some(peer.as_str()) {
            return Err(format!("step=accepted-view observed={row};needed=friend-{peer}").into());
        }
    }
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
    let (code, _) = friends(&mut a, vip::Operation::FriendRevoke, outgoing, "").await?;
    if code != 1 {
        return Err(format!("step=revoke observed=code:{code};needed=1").into());
    }
    if find_peer(&mut a, &name_b).await?.is_some() || find_peer(&mut b, &name_a).await?.is_some() {
        return Err("step=revoke observed=fixture-friend-still-listed".into());
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
    fn friendship_uses_the_fixture_peer_and_allows_unrelated_rows_after_revoke() {
        let unrelated = serde_json::json!({"name":"Other","state":"friend","handle":1});
        let fixture = serde_json::json!({"name":"Fixture","state":"incoming","handle":2});
        let rows = [unrelated.clone(), fixture];
        assert_eq!(peer_row(&rows, "Fixture").and_then(|row| row["handle"].as_u64()), Some(2));
        assert!(peer_row(&[unrelated], "Fixture").is_none());
    }
}
