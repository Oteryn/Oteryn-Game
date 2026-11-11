//! G15 analyser item and party hunt queries (capability 10, commands 734 and 735).
//! 1. Admit both provisioned characters; wait until they see each other, then form a party
//!    (invite, accept) through the party command.
//! 2. Query the item catalogue (734) for the gold coin items and carried items: names come from
//!    the server catalogue and the coin worth must be exactly 1, 100 and 10000 for the three coins.
//! 3. Query the party hunt analyser (735) from both members with a price list: both members'
//!    names (read-only SQL) must appear in each reply.
//! 4. Leave the party and log both out. No kills are made, so loot tracker totals stay zero.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::EntityRef;
use oteryn_dev_client::client_runtime::party::party::{PartyDisposition, PartyIntent};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::{analyser_catalogue as items, analyser_party as hunt};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(120), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "analyser-item-and-party-queries-validated".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G15.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut RowClient) -> TestResult {
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

const ANALYSER: u32 = 10;
const COINS: [(u32, u64); 3] = [(3031, 1), (3035, 100), (3043, 10_000)];

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client
        .world_entities()
        .ok_or("step=admit observed=no-entity-view")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "step=admit observed=own-entity-absent".into())
}

fn seen(client: &DevClientSession, identity: [u8; 16]) -> Option<EntityRef> {
    client
        .world_entities()?
        .iter()
        .find(|entry| entry.entity.identity == identity)
        .map(|entry| entry.entity)
}

async fn pump(a: &mut DevClientSession, b: &mut DevClientSession) -> TestResult {
    a.service_liveness(Duration::from_millis(25)).await?;
    b.service_liveness(Duration::from_millis(25)).await?;
    a.take_events();
    b.take_events();
    Ok(())
}

/// Waits until `a` sees `b`; returns `b` as `a` sees it.
async fn see_each_other(
    a: &mut DevClientSession,
    b: &mut DevClientSession,
) -> TestResult<EntityRef> {
    let target = own(b)?.identity;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    loop {
        if let Some(found) = seen(a, target) {
            return Ok(found);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "step=admit observed=other-player-not-in-view needed=two-players-in-view".into(),
            );
        }
        pump(a, b).await?;
    }
}

async fn name_of(clients: &SeamClients<'_>, index: usize) -> TestResult<String> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[index].as_slice())
    .fetch_one(&mut connection)
    .await?)
}

async fn party(
    client: &mut DevClientSession,
    intent: Option<PartyIntent>,
) -> TestResult<(
    PartyDisposition,
    oteryn_dev_client::client_runtime::party::party::PartyResult,
)> {
    use oteryn_dev_client::client_runtime::party as runtime;
    let command = match intent {
        Some(intent) => {
            runtime::PartyCommand::intent(intent).map_err(|e| format!("step=party encode={e:?}"))?
        }
        None => runtime::PartyCommand::query(),
    };
    let reply = runtime::send(client.session_mut(), &command).await?;
    let operation = runtime::party::decode_party_result(&reply.operation)
        .map_err(|e| format!("step=party operation={e:?}"))?;
    let view =
        runtime::party::decode_party_result(reply.view.as_deref().ok_or("step=party view-absent")?)
            .map_err(|e| format!("step=party view={e:?}"))?;
    Ok((operation.disposition, view))
}

fn coin_worth(index: &[(u32, u32)], item: u32) -> Option<u64> {
    index
        .iter()
        .find(|(reference, _)| *reference == item)
        .map(|(_, worth)| u64::from(*worth))
}

fn names_present(members: &[hunt::Member], names: &[&str]) -> bool {
    names.iter().all(|name| {
        members
            .iter()
            .any(|member| member.name.eq_ignore_ascii_case(name))
    })
}

async fn members(
    client: &mut DevClientSession,
    prices: &[hunt::Price],
) -> TestResult<Vec<hunt::Member>> {
    let payload = hunt::encode_request(prices)?;
    let reply = accepted(
        client,
        ANALYSER,
        hunt::COMMAND_TYPE_ANALYSER_PARTY_QUERY,
        &payload,
    )
    .await?;
    Ok(hunt::decode_reply(&reply)?)
}

async fn body(
    a: &mut DevClientSession,
    b: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    let b_seen = see_each_other(a, b).await?;
    let (reply, _) = party(a, Some(PartyIntent::Invite(b_seen))).await?;
    if reply != PartyDisposition::Ok {
        return Err(format!("step=invite observed={reply:?}").into());
    }
    let (_, invitation) = party(b, None).await?;
    let handle = invitation
        .invitations
        .first()
        .ok_or("step=invite observed=no-invitation-for-peer")?
        .handle;
    let (reply, _) = party(b, Some(PartyIntent::Accept(handle))).await?;
    if reply != PartyDisposition::Ok {
        return Err(format!("step=accept observed={reply:?}").into());
    }
    let (_, view) = party(a, None).await?;
    if view.members.len() < 2 {
        return Err(format!("step=party observed=members:{}", view.members.len()).into());
    }

    let index = provisioning::item_index()?;
    let mut worth = Vec::new();
    let mut ids = Vec::new();
    for (tibia, value) in COINS {
        let reference = provisioning::item_reference(&index, tibia)?.get();
        worth.push((reference, u32::try_from(value)?));
        ids.push(reference);
    }
    let carried: Vec<u32> = a
        .inventory()
        .map(|inventory| {
            inventory
                .entries
                .iter()
                .map(|e| e.item_definition_ref.get())
                .collect()
        })
        .unwrap_or_default();
    for reference in carried {
        if ids.len() < 8 && !ids.contains(&reference) {
            ids.push(reference);
        }
    }
    let payload = items::encode_request(&ids)?;
    let reply = accepted(
        a,
        ANALYSER,
        items::COMMAND_TYPE_ANALYSER_ITEM_QUERY,
        &payload,
    )
    .await?;
    let rows = items::decode_reply(&reply)?;
    if rows.len() != ids.len()
        || !ids
            .iter()
            .all(|id| rows.iter().any(|row| row.item_type == *id))
    {
        return Err(format!(
            "step=items observed=rows:{}-requested:{}",
            rows.len(),
            ids.len()
        )
        .into());
    }
    for row in &rows {
        let expected = coin_worth(&worth, row.item_type).unwrap_or(0);
        if row.name.is_empty() || row.coin_worth != expected {
            return Err(format!(
                "step=items observed=item:{}-name:{}-worth:{}-expected:{expected}",
                row.item_type, row.name, row.coin_worth
            )
            .into());
        }
    }

    let prices: Vec<hunt::Price> = ids
        .iter()
        .map(|item| hunt::Price {
            item: *item,
            loot: Some(1),
            supply: Some(1),
        })
        .collect();
    let a_name = name_of(clients, 0).await?;
    let b_name = name_of(clients, 1).await?;
    for (who, client) in [("leader", &mut *a), ("member", &mut *b)] {
        let rows = members(client, &prices).await?;
        if !names_present(&rows, &[a_name.as_str(), b_name.as_str()]) {
            return Err(format!(
                "step=party-analyser observed={who}-names:{:?} needed={a_name}+{b_name}",
                rows.iter().map(|m| m.name.as_str()).collect::<Vec<_>>()
            )
            .into());
        }
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut a = row_connect_character(clients, 0x78, 0).await?;
    let mut b = match row_connect_character(clients, 0x79, 1).await {
        Ok(b) => b,
        Err(error) => {
            finish(&mut a).await.ok();
            return Err(error);
        }
    };
    let outcome = body(&mut a, &mut b, clients).await;
    party(&mut b, Some(PartyIntent::Leave)).await.ok();
    party(&mut a, Some(PartyIntent::Leave)).await.ok();
    let closed_b = finish(&mut b).await;
    let closed_a = finish(&mut a).await;
    outcome?;
    closed_b?;
    closed_a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }

    #[test]
    fn names_are_matched_case_insensitively() {
        let member = |name: &str| hunt::Member {
            handle: 1,
            name: name.to_owned(),
            available: true,
            incomplete: false,
            elapsed_seconds: 0,
            kills: 0,
            xp: 0,
            raw_xp: 0,
            loot_items: 0,
            loot_value: 0,
            supplies: 0,
            supply_cost: 0,
            unpriced_loot: 0,
            unpriced_supplies: 0,
            damage: 0,
            healing: 0,
            needed_prices: Vec::new(),
        };
        let rows = [member("Alpha"), member("beta")];
        assert!(names_present(&rows, &["alpha", "Beta"]));
        assert!(!names_present(&rows, &["gamma"]));
    }

    #[test]
    fn coin_worth_reads_the_index() {
        assert_eq!(coin_worth(&[(7, 100)], 7), Some(100));
        assert_eq!(coin_worth(&[(7, 100)], 8), None);
    }
}
