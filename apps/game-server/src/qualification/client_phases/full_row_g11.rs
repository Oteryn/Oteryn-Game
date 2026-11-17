//! G11 imbuement details query (capability 447, command 731).
//! 1. Admit the node-boot character; require capability 447 in the negotiated session.
//! 2. Query every carried item handle through the same payload the imbuement panel sends; a
//!    refusal for an item without an imbuement allowance is the correct answer and is skipped.
//! 3. For the first item with quotes, validate the echoed handle, the (family, tier) ordering and
//!    the bounded success chance, which only the server's imbuement catalogue can produce.
//! 4. Log out, freshly admit and re-query the same durable item instance: the quotes must be equal.
//!
//! The row never grants items; with no imbuable item carried it fails naming that precondition.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::ItemHandle;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::imbuement_details as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "imbuement-quotes-validated-and-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G11.full status={status} seconds={:.3} evidence={detail}",
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

async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

const CAPABILITY: u32 = wire::CAPABILITY_IMBUEMENT_DETAILS_V1;

fn quotes_ok(quotes: &[wire::Quote]) -> bool {
    !quotes.is_empty()
        && quotes
            .windows(2)
            .all(|pair| (pair[0].family, pair[0].tier) < (pair[1].family, pair[1].tier))
        && quotes.iter().all(|quote| {
            quote.success_percent <= 100 && quote.materials.len() <= wire::MAX_MATERIALS
        })
}

#[derive(Debug)]
struct Carried {
    handle: ItemHandle,
    definition: u32,
    id: [u8; 16],
}

#[derive(Debug)]
struct Evidence {
    item: Carried,
    quotes: Vec<wire::Quote>,
}

async fn carried(client: &DevClientSession, clients: &SeamClients<'_>) -> TestResult<Vec<Carried>> {
    use sqlx::Connection;
    let inventory = client.inventory().ok_or("step=inventory observed=no-inventory-view")?;
    let index = provisioning::item_index()?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let rows: Vec<(Vec<u8>, String, String, i64)> = sqlx::query_as(
        "SELECT uuid_send(i.item_instance_id), i.definition_production_key, i.definition_revision_ref, i.quantity::bigint          FROM game_item_container_slots s          JOIN game_item_container_entries e ON (e.parent_item_instance_id,e.world_id)=(s.item_instance_id,s.world_id)          JOIN game_item_instances i ON (i.item_instance_id,i.world_id)=(e.item_instance_id,e.world_id)          WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid AND i.lifecycle=1          ORDER BY e.placement_ordinal DESC LIMIT $3",
    ).bind(clients.characters[1].as_slice()).bind(clients.world.as_slice())
        .bind(i64::try_from(oteryn_protocol_oteryn::item_view::MAX_CHARACTER_INVENTORY_ITEMS)?)
        .fetch_all(&mut database).await?;
    if rows.len() != inventory.entries.len() {
        return Err("step=inventory observed=wire-and-durable-entry-count-disagree".into());
    }
    // Domain 9 preserves the direct backpack order (descending durable placement ordinal).
    inventory.entries.iter().zip(rows).map(|(entry, (id, key, revision, quantity))| {
        let definition = entry.item_definition_ref;
        if index.definition_at(definition) != Some((key.as_str(), revision.as_str()))
            || i64::from(entry.count.get()) != quantity
        {
            return Err("step=inventory observed=wire-and-durable-entry-disagree".into());
        }
        let id = id.try_into().map_err(|_| "step=inventory observed=invalid-instance-id")?;
        Ok(Carried { handle: entry.handle, definition: definition.get(), id })
    }).collect()
}

fn item_matches(definition: u32, id: &[u8], wanted_definition: u32, wanted_id: &[u8]) -> bool {
    definition == wanted_definition && id == wanted_id
}

/// Rebind the same durable instance to the handle issued on this admission.
async fn quotes_for(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    only: Option<&Carried>,
) -> TestResult<(Option<Evidence>, String)> {
    let mut observed = Vec::new();
    for item in carried(client, clients).await? {
        let Carried { handle, definition, id } = item;
        if only.is_some_and(|wanted| !item_matches(definition, &id, wanted.definition, &wanted.id)) {
            continue;
        }
        let (status, payload) = send(
            client,
            CAPABILITY,
            wire::COMMAND_TYPE_IMBUEMENT_DETAILS,
            &wire::encode_query(handle),
        )
        .await?;
        if status != CommandStatus::Accepted {
            observed.push(format!("{definition}:{status:?}"));
            continue;
        }
        let reply = wire::decode_reply(&payload)
            .map_err(|error| format!("step=decode observed={error:?}"))?;
        if reply.item != handle {
            return Err("step=query observed=reply-for-another-item".into());
        }
        if reply.quotes.is_empty() {
            observed.push(format!("{definition}:no-quotes"));
            continue;
        }
        if !quotes_ok(&reply.quotes) {
            return Err(format!("step=quotes observed=invalid:{:?}", reply.quotes).into());
        }
        return Ok((
            Some(Evidence { item, quotes: reply.quotes }),
            observed.join(","),
        ));
    }
    Ok((None, observed.join(",")))
}

async fn body(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<Evidence> {
    match quotes_for(client, clients, None).await? {
        (Some(found), _) => Ok(found),
        (None, observed) => Err(format!(
            "step=query observed={observed} needed=carried-item-with-imbuement-allowance"
        )
        .into()),
    }
}

async fn verify(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    data: &Evidence,
) -> TestResult {
    match quotes_for(client, clients, Some(&data.item)).await? {
        (Some(found), _) if found.quotes == data.quotes => Ok(()),
        (found, observed) => Err(format!(
            "step=relog observed={found:?}/{observed} needed=quotes-unchanged-after-fresh-admission"
        )
        .into()),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x70).await?;
    let outcome = verdict::observe(body(&mut client, clients)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let data = outcome?;
    closed?;
    let mut fresh = connect_world(clients, 0x71).await?;
    let checked = verdict::observe(verify(&mut fresh, clients, &data)).await;
    let closed = verdict::observe(finish(&mut fresh)).await;
    checked?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imbuement_fresh_lookup_rejects_a_copy_of_the_original_item() {
        assert!(!item_matches(7, &[2; 16], 7, &[1; 16]));
        assert!(item_matches(7, &[1; 16], 7, &[1; 16]));
    }

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
    fn quotes_must_be_ordered_and_bounded() {
        let quote = |family, tier, success_percent| wire::Quote {
            family,
            tier,
            gold: 1,
            requires_premium: false,
            recipe_available: true,
            success_percent,
            materials: Vec::new(),
        };
        assert!(quotes_ok(&[
            quote(1, 1, 90),
            quote(1, 2, 60),
            quote(2, 1, 100)
        ]));
        assert!(!quotes_ok(&[]));
        assert!(!quotes_ok(&[quote(1, 2, 90), quote(1, 1, 90)]));
        assert!(!quotes_ok(&[quote(1, 1, 101)]));
    }
}
