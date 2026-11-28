//! N03 client replay capture: deposit a carried item at an authored depot locker, then capture the
//! real Depot Search holdings list (every page up to the item) and the RETRIEVE(one) reply. Client
//! owner: `activate_shared` in `apps/client/src/features/0153-depot-search/native.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_view::ItemEntry;
use oteryn_protocol_oteryn::{CommandStatus, depot, depot_search as wire};

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    std::fs::File::create_new(root.join(name))
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|error| format!("N03_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}").into())
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
    let envelope = capture_envelope::encode(
        client.join_snapshot().game_session_id.as_bytes(),
        response.command_id,
        response.server_sequence,
        response.status as u64,
        &response.payload,
    )?;
    Ok((reply, envelope))
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

fn carried_count(client: &DevClientSession, item: ItemEntry) -> TestResult<u64> {
    let inventory = client.inventory().ok_or("N03 inventory snapshot missing")?;
    Ok(inventory
        .main_backpack
        .iter()
        .chain(&inventory.entries)
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

async fn wait_carried(client: &mut DevClientSession, item: ItemEntry, expected: u64) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let actual = carried_count(client, item)?;
        if actual == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("N03 inventory delta expected={expected} actual={actual}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

async fn assert_no_holdings(client: &mut DevClientSession, client_id: u32) -> TestResult {
    let (reply, _) = search_command(client, wire::Request::Search { client_id }).await?;
    if reply.outcome != wire::Outcome::NotFound || !reply.results.is_empty() || reply.retrieved != 0 {
        return Err(format!("N03 expected absent holdings item={client_id} reply={reply:?}").into());
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let outcome = client.session_mut().logout().await?;
    if outcome.status != CommandStatus::Accepted
        || outcome.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("N03 logout refused: {outcome:?}").into());
    }
    Ok(())
}

async fn walk_to_depot(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    let mut step_wait_ms = 0;
    for &direction in path {
        if step_wait_ms != 0 {
            client.service_liveness(Duration::from_millis(u64::from(step_wait_ms))).await?;
        }
        let start = tile(client.world_spatial().actor_position)?;
        let landing = destination(world, start, direction).ok_or("N03 invalid depot route")?;
        let mut outcome = client.step(direction).await?;
        if outcome.disposition == StepDisposition::Blocked {
            let occupant = client.world_entities().and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Creature
                        && tile(entity.position).ok() == Some(landing)
                }).map(|entity| entity.entity.identity)
            }).ok_or_else(|| format!("BLOCKER-SERVER N03 blocked without visible creature at {landing:?}: {outcome:?}"))?;
            evidence(&format!("N03_MOVE_BLOCKED occupant={occupant:?} at={landing:?} frame={outcome:?}"));
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            loop {
                client.service_liveness(Duration::from_millis(100)).await?;
                let still_there = client.world_entities().is_some_and(|entities| {
                    entities.iter().any(|entity| entity.entity.identity == occupant
                        && tile(entity.position).ok() == Some(landing))
                });
                if !still_there {
                    evidence(&format!("N03_MOVE_CLEARED occupant={occupant:?} at={landing:?}"));
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("N03 creature {occupant:?} remained at {landing:?}").into());
                }
            }
            outcome = client.step(direction).await?;
        }
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != landing
        {
            return Err(format!("N03 step {direction:?} expected={landing:?}: {outcome:?}").into());
        }
        step_wait_ms = outcome.step_wait_ms;
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let capture_root = required("E2E_ALL_CAPTURE_DIR")?;
    let capture_root = Path::new(&capture_root);
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
    walk_to_depot(&mut client, world, &path).await?;
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
    if item.count.get() != 1 || carried_count(&client, item)? != 1 {
        return Err("N03 expected exactly one carried unit before deposit".into());
    }
    assert_no_holdings(&mut client, client_id).await?;
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
    wait_carried(&mut client, item, 0).await?;

    // Preserve the full client pagination, even when the item appears before the last page.
    let mut pages = Vec::new();
    let mut after = 0;
    let mut row = None;
    let listed = loop {
        let (page, page_envelope) = search_command(&mut client, wire::Request::Open { after }).await?;
        if let Some(found) = page.results.iter().find(|row| row.client_id == client_id)
            && row.replace(found.clone()).is_some()
        {
            return Err("N03 item repeated across holdings pages".into());
        }
        pages.push(page_envelope);
        if page.outcome == wire::Outcome::Listed {
            break page;
        }
        if page.outcome != wire::Outcome::More || pages.len() >= 64 {
            return Err(format!("N03 holdings pagination did not finish: {:?}", page.outcome).into());
        }
        let next = page.results.last().ok_or("empty More page")?.client_id;
        if next <= after {
            return Err("N03 holdings cursor did not advance".into());
        }
        after = next;
    };
    let row = row.ok_or_else(|| format!("deposited item {client_id} not in the holdings list"))?;
    if (row.depot, row.stash, row.inbox) != (1, 0, 0) {
        return Err(format!("N03 expected one depot unit after deposit: {row:?}").into());
    }
    let (retrieved, retrieved_envelope) = search_command(
        &mut client,
        wire::Request::Retrieve {
            client_id,
            all: false,
        },
    )
    .await?;
    if retrieved.outcome != wire::Outcome::Retrieved || retrieved.retrieved != 1 {
        return Err(format!("retrieve-one reply {retrieved:?}").into());
    }
    let tiers = row
        .tiers
        .iter()
        .map(|tier| format!("{}x{}", tier.tier, tier.count))
        .collect::<Vec<_>>()
        .join(",");
    let original_session = client.join_snapshot().game_session_id;
    let capture_id = uuid_text(original_session.as_bytes());
    let expected = format!(
        "session={capture_id}\nitem={client_id}\npages={}\nlisted={:?}\nrow={}|{}|{}|{tiers}\nretrieve={:?}|{}\n",
        pages.len(),
        listed.outcome,
        row.depot,
        row.stash,
        row.inbox,
        retrieved.outcome,
        retrieved.retrieved
    );
    wait_carried(&mut client, item, 1).await?;
    assert_no_holdings(&mut client, client_id).await?;
    let locker_position = client.world_spatial().actor_position;
    logout(&mut client).await?;
    drop(client);

    let mut fresh = connect_character(clients, 0xad, 1).await?;
    if fresh.join_snapshot().game_session_id == original_session
        || fresh.world_spatial().actor_position != locker_position
        || carried_count(&fresh, item)? != 1
    {
        return Err("N03 relog did not restore locker position and exactly one carried unit".into());
    }
    assert_no_holdings(&mut fresh, client_id).await?;
    logout(&mut fresh).await?;
    evidence("N03_DURABILITY carried_before=1 carried_after_deposit=0 depot_after_deposit=1 carried_after_retrieve=1 relog_carried=1 relog_depot=0 fresh_session=true");

    for (index, envelope) in pages.iter().enumerate() {
        capture(
            capture_root,
            &format!("n03-list-{index}.json"),
            envelope,
        )?;
    }
    capture(
        capture_root,
        "n03-retrieve.json",
        &retrieved_envelope,
    )?;
    capture(capture_root, "n03-expected.txt", expected.as_bytes())?;
    Ok(format!(
        "session={capture_id} item={client_id} pages={} depot={} retrieved={}",
        pages.len(),
        row.depot,
        retrieved.retrieved
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N03.client status=CAPTURED seconds={:.3} evidence={observed}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N03.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=N03.client status=FAIL evidence=timeout"),
    }
}
