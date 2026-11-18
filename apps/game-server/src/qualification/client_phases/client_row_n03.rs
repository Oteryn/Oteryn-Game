//! N03 client replay capture: deposit a carried item at an authored depot locker, then capture the
//! real Depot Search holdings list (every page up to the item) and the RETRIEVE(one) reply. Client
//! owner: `activate_shared` in `apps/client/src/features/0153-depot-search/native.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::{CommandStatus, depot, depot_search as wire};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "N03_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
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
    let reply =
        wire::decode_reply(&response.payload).map_err(|error| format!("decode {error:?}"))?;
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
        .gated_command(
            depot::CAPABILITY_DEPOT_V1,
            depot::COMMAND_TYPE_DEPOT_INTENT,
            &payload,
        )
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

async fn wait_carried(
    client: &mut DevClientSession,
    handle: ItemHandle,
    expected: bool,
) -> TestResult {
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

fn carried_quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("carried inventory missing")?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == reference)
        .map(|item| u64::from(item.count.get()))
        .sum())
}

fn deposited_stock(before: u32, after: u32, quantity: u32) -> bool {
    before.checked_add(quantity) == Some(after)
}

fn restored_stock(
    receipt: u32,
    carried_before: u64,
    carried_after: u64,
    depot_unchanged: bool,
) -> bool {
    receipt == 1 && carried_before == carried_after && depot_unchanged
}

#[test]
fn depot_requires_deposit_delta_and_confirmed_restoration() {
    assert!(!deposited_stock(10, 10, 1));
    assert!(deposited_stock(10, 11, 1));
    assert!(!restored_stock(1, 5, 4, true));
    assert!(!restored_stock(1, 5, 5, false));
    assert!(restored_stock(1, 5, 5, true));
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
    let mut client = connect_character(clients, 0xac, 1).await?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("no reachable authored depot locker")?;
    walk(&mut client, world, &path).await?;
    let entries = client
        .inventory()
        .ok_or("no carried inventory")?
        .entries
        .clone();
    let mut selected = None;
    for item in entries.into_iter().filter(|item| item.count.get() == 1) {
        let Some((key, _)) = definitions.definition_at(item.item_definition_ref) else {
            continue;
        };
        let Some(client_id) = key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u32>().ok())
        else {
            continue;
        };
        let (baseline, _) =
            search_command(&mut client, wire::Request::Search { client_id }).await?;
        match (baseline.outcome, baseline.results.as_slice()) {
            (wire::Outcome::NotFound, []) => {
                selected = Some((item, client_id));
                break;
            }
            (wire::Outcome::Found, [row]) if row.client_id == client_id => {}
            _ => return Err(format!("unexpected holdings baseline: {baseline:?}").into()),
        }
    }
    let (item, client_id) =
        selected.ok_or("no single carried item absent from depot, stash and inbox")?;
    let carried_before = carried_quantity(&client, item.item_definition_ref)?;
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

    // List the holdings page by page, as the client does (first page, then next page after the
    // last row), until the deposited item is listed.
    let mut pages = Vec::new();
    let mut after = 0;
    let (listed, row) = loop {
        let (page, page_bytes) = search_command(&mut client, wire::Request::Open { after }).await?;
        let found = page
            .results
            .iter()
            .find(|row| row.client_id == client_id)
            .cloned();
        pages.push(page_bytes);
        if let Some(row) = found {
            break (page, row);
        }
        if page.outcome != wire::Outcome::More || pages.len() >= 64 {
            return Err(format!("deposited item {client_id} not in the holdings list").into());
        }
        after = page.results.last().ok_or("empty More page")?.client_id;
    };
    if !deposited_stock(0, row.depot, item.count.get()) || row.stash != 0 || row.inbox != 0 {
        return Err(format!("listed row does not reflect the isolated deposit: {row:?}").into());
    }
    let (retrieved, retrieved_bytes) = search_command(
        &mut client,
        wire::Request::Retrieve {
            client_id,
            all: false,
        },
    )
    .await?;
    if retrieved.outcome != wire::Outcome::Retrieved || retrieved.retrieved == 0 {
        return Err(format!("retrieve-one reply {retrieved:?}").into());
    }
    tokio::time::timeout(Duration::from_secs(10), async {
        while carried_quantity(&client, item.item_definition_ref)? != carried_before {
            client.service_liveness(Duration::from_millis(25)).await?;
            client.take_events();
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await
    .map_err(|_| "step=retrieve inventory did not return to baseline")??;
    let (remaining, _) = search_command(&mut client, wire::Request::Search { client_id }).await?;
    if !restored_stock(
        retrieved.retrieved,
        carried_before,
        carried_quantity(&client, item.item_definition_ref)?,
        remaining.outcome == wire::Outcome::NotFound && remaining.results.is_empty(),
    ) {
        return Err(format!(
            "depot restoration unconfirmed: retrieve={retrieved:?}; remaining={remaining:?}"
        )
        .into());
    }
    let tiers = row
        .tiers
        .iter()
        .map(|tier| format!("{}x{}", tier.tier, tier.count))
        .collect::<Vec<_>>()
        .join(",");
    let expected = format!(
        "item={client_id}\npages={}\nlisted={:?}\nrow={}|{}|{}|{tiers}\nretrieve={:?}|{}\n",
        pages.len(),
        listed.outcome,
        row.depot,
        row.stash,
        row.inbox,
        retrieved.outcome,
        retrieved.retrieved
    );
    for (index, bytes) in pages.iter().enumerate() {
        capture(&format!("n03-list-{index}.bin"), bytes);
    }
    capture("n03-retrieve.bin", &retrieved_bytes);
    capture("n03-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!(
        "item={client_id} pages={} depot={} retrieved={}",
        pages.len(),
        row.depot,
        retrieved.retrieved
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N03.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N03.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=N03.client status=FAIL evidence=timeout"),
    }
}
