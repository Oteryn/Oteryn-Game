//! G03: depot search result list and retrieval (apps/client 0153-depot-search).
//! 1. Walk to an authored depot locker, browse a depot box and deposit one carried item.
//! 2. Send OPEN: expect LISTED carrying a row for the deposited item with its depot count.
//! 3. Send RETRIEVE for one unit from the listed row: expect RETRIEVED(1) and a carried-count
//!    delta of exactly one unit; the list must now show one unit fewer.
//! 4. Retrieve the remainder (all=true) and require the original carried count restored and an
//!    empty row for that item in a fresh OPEN list.
//! 5. Log out, freshly admit, walk to the locker, OPEN again: the durable list has no row for
//!    the item and the carried count is unchanged.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{depot, depot_search as wire};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(180), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "list-and-retrieval-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G03.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn command(client: &mut DevClientSession, request: wire::Request) -> TestResult<wire::Reply> {
    let payload = wire::encode_request(request).map_err(|error| format!("encode:{error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DEPOT_SEARCH_V1,
            wire::COMMAND_TYPE_DEPOT_SEARCH,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("command-status:{:?}", response.status).into());
    }
    wire::decode_reply(&response.payload).map_err(|error| format!("decode:{error:?}").into())
}

fn carried_count(client: &DevClientSession, definition: std::num::NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .main_backpack
            .iter()
            .chain(&inventory.entries)
            .chain(inventory.equipment.iter().map(|entry| &entry.item))
            .filter(|entry| entry.item_definition_ref == definition)
            .map(|entry| u64::from(entry.count.get()))
            .sum()
    })
}

async fn wait_count(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    expected: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while carried_count(client, definition) != expected {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=inventory-delta observed=count:{};needed={expected}",
                carried_count(client, definition)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

async fn depot_intent(
    client: &mut DevClientSession,
    intent: depot::DepotIntent,
) -> TestResult<depot::DepotReply> {
    let payload =
        depot::encode_depot_intent(intent).map_err(|error| format!("depot-encode:{error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            depot::CAPABILITY_DEPOT_V1,
            depot::COMMAND_TYPE_DEPOT_INTENT,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("depot-status:{:?}", response.status).into());
    }
    depot::decode_depot_reply(&response.payload)
        .map_err(|error| format!("depot-decode:{error:?}").into())
}

async fn to_locker(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| near_locker(world, at))
        .ok_or("step=locker observed=no-reachable-authored-depot-locker")?;
    walk(client, world, &path).await
}

async fn logout(client: &mut RowClient) -> TestResult {
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

fn listed(reply: &wire::Reply, client_id: u32) -> Option<u64> {
    reply
        .results
        .iter()
        .find(|row| row.client_id == client_id)
        .map(|row| u64::from(row.depot) + u64::from(row.stash) + u64::from(row.inbox))
}

async fn open_list(client: &mut DevClientSession) -> TestResult<wire::Reply> {
    let reply = command(client, wire::Request::Open { after: 0 }).await?;
    if !matches!(reply.outcome, wire::Outcome::Listed | wire::Outcome::More) {
        return Err(format!("step=open observed=outcome:{:?}", reply.outcome).into());
    }
    Ok(reply)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let definitions = provisioning::item_index()?;
    let mut client = row_connect_character(clients, 0xc3, 1).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_DEPOT_SEARCH_V1)
    {
        return Err("step=admit observed=capability-153-not-negotiated".into());
    }
    to_locker(&mut client, world).await?;
    let item = client
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .copied()
        .ok_or("step=admit observed=no-carried-item")?;
    let (key, _) = definitions
        .definition_at(item.item_definition_ref)
        .ok_or("step=admit observed=carried-item-definition-unknown")?;
    let client_id: u32 = key
        .strip_prefix("oteryn:item.tibia.i")
        .ok_or("step=admit observed=carried-item-has-no-native-client-id")?
        .parse()?;
    let original = carried_count(&client, item.item_definition_ref);
    let stack = u64::from(item.count.get());
    let browse = depot_intent(
        &mut client,
        depot::DepotIntent {
            town: None,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Browse,
        },
    )
    .await?;
    if browse.outcome != depot::DepotOutcome::Opened || browse.town.is_none() {
        return Err(format!("step=browse observed={:?}", browse.outcome).into());
    }
    let moved = depot_intent(
        &mut client,
        depot::DepotIntent {
            town: browse.town,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Deposit(item.handle),
        },
    )
    .await?;
    if moved.outcome != depot::DepotOutcome::Moved {
        return Err(format!("step=deposit observed={:?}", moved.outcome).into());
    }
    wait_count(&mut client, item.item_definition_ref, original - stack).await?;

    let list = open_list(&mut client).await?;
    let held = listed(&list, client_id).ok_or("step=open observed=deposited-item-not-listed")?;
    if held < stack {
        return Err(format!("step=open observed=count:{held};needed>={stack}").into());
    }
    let one = command(
        &mut client,
        wire::Request::Retrieve {
            client_id,
            all: false,
        },
    )
    .await?;
    if one.outcome != wire::Outcome::Retrieved || one.retrieved != 1 {
        return Err(format!(
            "step=retrieve-one observed=outcome:{:?},retrieved:{}",
            one.outcome, one.retrieved
        )
        .into());
    }
    wait_count(&mut client, item.item_definition_ref, original - stack + 1).await?;
    let after_one = open_list(&mut client).await?;
    if listed(&after_one, client_id).unwrap_or(0) != held - 1 {
        return Err(format!(
            "step=list-after-one observed={:?};needed={}",
            listed(&after_one, client_id),
            held - 1
        )
        .into());
    }
    if held > 1 {
        let rest = command(
            &mut client,
            wire::Request::Retrieve {
                client_id,
                all: true,
            },
        )
        .await?;
        if rest.outcome != wire::Outcome::Retrieved || u64::from(rest.retrieved) != held - 1 {
            return Err(format!(
                "step=retrieve-rest observed=outcome:{:?},retrieved:{}",
                rest.outcome, rest.retrieved
            )
            .into());
        }
    }
    wait_count(
        &mut client,
        item.item_definition_ref,
        original - stack + held,
    )
    .await?;
    if listed(&open_list(&mut client).await?, client_id).is_some() {
        return Err("step=list-empty observed=row-still-listed".into());
    }
    let restored = carried_count(&client, item.item_definition_ref);
    logout(&mut client).await?;
    drop(client);

    let mut fresh = row_connect_character(clients, 0xc4, 1).await?;
    to_locker(&mut fresh, world).await?;
    let durable = open_list(&mut fresh).await?;
    let carried = carried_count(&fresh, item.item_definition_ref);
    logout(&mut fresh).await?;
    if listed(&durable, client_id).is_some() || carried != restored {
        return Err(format!(
            "step=fresh observed=listed:{:?},carried:{carried};needed=none,{restored}",
            listed(&durable, client_id)
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listed_sums_every_custody_bucket_of_the_requested_row() {
        let reply = wire::Reply {
            outcome: wire::Outcome::Listed,
            retrieved: 0,
            results: vec![wire::SearchResult {
                client_id: 7,
                depot: 2,
                stash: 3,
                inbox: 1,
                tiers: Vec::new(),
            }],
        };
        assert_eq!(listed(&reply, 7), Some(6));
        assert_eq!(listed(&reply, 8), None);
    }
}
