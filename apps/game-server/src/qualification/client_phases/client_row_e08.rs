//! E08 client replay capture: deposit a carried item at an authored depot locker, then capture the
//! real Depot Search SEARCH, DETAIL, RETRIEVE(all) and empty-RETRIEVE replies. Client owner:
//! `apply_result` in `crates/client-ui/src/features/0153-depot-search/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::{CommandStatus, depot, depot_search as wire};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("E08_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn search_command(
    client: &mut DevClientSession,
    request: wire::Request,
) -> TestResult<(wire::Reply, Vec<u8>)> {
    let payload = wire::encode_request(request).map_err(|error| format!("encode {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DEPOT_SEARCH_V1,
            wire::COMMAND_TYPE_DEPOT_SEARCH,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("depot search status {:?}", response.status).into());
    }
    let reply = wire::decode_reply(&response.payload).map_err(|error| format!("decode {error:?}"))?;
    Ok((reply, response.payload))
}

async fn depot_intent(
    client: &mut DevClientSession,
    intent: depot::DepotIntent,
) -> TestResult<depot::DepotReply> {
    let payload =
        depot::encode_depot_intent(intent).map_err(|error| format!("depot encode {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(depot::CAPABILITY_DEPOT_V1, depot::COMMAND_TYPE_DEPOT_INTENT, &payload)
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("depot status {:?}", response.status).into());
    }
    depot::decode_depot_reply(&response.payload)
        .map_err(|error| format!("depot decode {error:?}").into())
}

fn carried(client: &DevClientSession, handle: ItemHandle) -> bool {
    client.inventory().is_some_and(|inventory| {
        inventory
            .main_backpack
            .iter()
            .chain(&inventory.entries)
            .chain(inventory.equipment.iter().map(|entry| &entry.item))
            .any(|entry| entry.handle == handle)
    })
}

async fn wait_carried(client: &mut DevClientSession, handle: ItemHandle, expected: bool) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while carried(client, handle) != expected {
        if tokio::time::Instant::now() >= deadline {
            return Err("inventory delta timeout".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let definitions = provisioning::item_index()?;
    let mut client = connect_character(clients, 0x3b, 1).await?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("no reachable authored depot locker")?;
    walk(&mut client, world, &path).await?;
    let item = client
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .copied()
        .ok_or("no carried item to deposit")?;
    let (key, _) = definitions
        .definition_at(item.item_definition_ref)
        .ok_or("carried item definition unknown")?;
    let client_id: u32 = key
        .strip_prefix("oteryn:item.tibia.i")
        .ok_or("carried item has no native client id")?
        .parse()?;
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
        return Err("depot browse did not open".into());
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
        return Err(format!("deposit outcome {:?}", moved.outcome).into());
    }
    wait_carried(&mut client, item.handle, false).await?;

    let (found, found_bytes) = search_command(&mut client, wire::Request::Search { client_id }).await?;
    let row = found.results.first().ok_or("search returned no row")?;
    let tier = row.tiers.first().ok_or("search row has no tier")?;
    if found.outcome != wire::Outcome::Found
        || found.retrieved != 0
        || row.client_id != client_id
        || row.depot == 0
    {
        return Err(format!("search reply {found:?}").into());
    }
    let (tier_number, tier_count) = (tier.tier, tier.count);
    let total = row.depot + row.stash + row.inbox;
    let (detail, detail_bytes) = search_command(
        &mut client,
        wire::Request::Detail {
            client_id,
            tier: tier_number,
        },
    )
    .await?;
    if detail.outcome != wire::Outcome::Detailed || detail.results.len() != 1 {
        return Err(format!("detail reply {detail:?}").into());
    }
    let (retrieved, retrieved_bytes) = search_command(
        &mut client,
        wire::Request::Retrieve {
            client_id,
            all: true,
        },
    )
    .await?;
    if retrieved.outcome != wire::Outcome::Retrieved || retrieved.retrieved != total {
        return Err(format!("retrieve reply {retrieved:?}").into());
    }
    wait_carried(&mut client, item.handle, true).await?;
    let (empty, empty_bytes) = search_command(
        &mut client,
        wire::Request::Retrieve {
            client_id,
            all: true,
        },
    )
    .await?;
    if empty.outcome != wire::Outcome::NotFound || empty.retrieved != 0 {
        return Err(format!("empty retrieve reply {empty:?}").into());
    }

    let first = &found.results[0];
    let expected = format!(
        "item={client_id}\ntier={tier_number}\nsearch={:?}|{}|{}|{}|{tier_number}x{tier_count}\n\
         detail={:?}\nretrieve={:?}|{}\nempty={:?}|{}\n",
        found.outcome,
        first.depot,
        first.stash,
        first.inbox,
        detail.outcome,
        retrieved.outcome,
        retrieved.retrieved,
        empty.outcome,
        empty.retrieved
    );
    capture("e08-search.bin", &found_bytes);
    capture("e08-detail.bin", &detail_bytes);
    capture("e08-retrieve.bin", &retrieved_bytes);
    capture("e08-empty.bin", &empty_bytes);
    capture("e08-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!("item={client_id} depot={} retrieved={}", first.depot, retrieved.retrieved))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E08.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E08.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E08.client status=FAIL evidence=timeout"),
    }
}
