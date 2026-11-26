//! E08 client replay capture: deposit a carried item at an authored depot locker, then capture the
//! real Depot Search SEARCH, DETAIL, RETRIEVE(all) and empty-RETRIEVE replies. Client owner:
//! `apply_result` in `crates/client-ui/src/features/0153-depot-search/protocol.rs`.
use super::full_row_e08::{carried_count, last_carried_handle, logout, restored, to_locker};
use super::*;
use oteryn_dev_client::{
    DevClientSession, client_runtime::server_mechanic::ServerMechanicResponse,
    client_ui::depot_search as panel,
};
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::{CommandStatus, depot, depot_search as wire};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::fs::write(Path::new(&root).join(name), bytes)?;
    }
    Ok(())
}

async fn search_command(
    client: &mut DevClientSession,
    request: wire::Request,
) -> TestResult<(wire::Reply, ServerMechanicResponse)> {
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
    Ok((reply, response))
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CapturedResponse {
    session: String,
    command_id: u64,
    server_sequence: u64,
    status: u32,
    payload: Vec<u8>,
}

fn replay(
    directory: &Path,
    session: &str,
    client_id: u32,
    tier: u8,
    units: u32,
    responses: [ServerMechanicResponse; 4],
) -> TestResult {
    let path = directory.join("e08-command-results.json");
    let captured = responses.map(|reply| CapturedResponse {
        session: session.to_owned(),
        command_id: reply.command_id,
        server_sequence: reply.server_sequence,
        status: reply.status as u32,
        payload: reply.payload,
    });
    std::fs::write(&path, serde_json::to_vec(&captured)?)?;
    let frames: [CapturedResponse; 4] = serde_json::from_slice(&std::fs::read(path)?)?;
    let mut view = panel::DepotSearchPanel {
        selected: true,
        item_id: client_id,
        tier,
        ..Default::default()
    };
    let heading = format!(
        "Depot Search\nItem id: {client_id}\nSelected tier: {tier}\nRetrieve one retrieves one unit. Retrieve all includes every tier of the selected item."
    );
    let mut previous = None;
    for (frame, (request, outcome, retrieved, has_row)) in frames.into_iter().zip([
        (
            wire::Request::Search { client_id },
            wire::Outcome::Found,
            0,
            true,
        ),
        (
            wire::Request::Detail { client_id, tier },
            wire::Outcome::Detailed,
            0,
            true,
        ),
        (
            wire::Request::Retrieve {
                client_id,
                all: true,
            },
            wire::Outcome::Retrieved,
            units,
            false,
        ),
        (
            wire::Request::Retrieve {
                client_id,
                all: true,
            },
            wire::Outcome::NotFound,
            0,
            false,
        ),
    ]) {
        if frame.session != session
            || frame.status != CommandStatus::Accepted as u32
            || frame.command_id == 0
            || frame.server_sequence == 0
            || previous.is_some_and(|(command, sequence): (u64, u64)| {
                command.checked_add(1) != Some(frame.command_id)
                    || sequence >= frame.server_sequence
            })
        {
            return Err("invalid captured depot search session or sequence".into());
        }
        previous = Some((frame.command_id, frame.server_sequence));
        let payload = wire::encode_request(request).map_err(|error| format!("{error:?}"))?;
        let issued = match request {
            wire::Request::Search { .. } => panel::search(&mut view),
            wire::Request::Detail { .. } => panel::detail(&mut view),
            _ => panel::request(&mut view, panel::command(payload.clone())),
        }
        .ok_or("client depot command not issued")?;
        if !view.pending || issued.payload != payload {
            return Err("client depot request did not enter the expected pending state".into());
        }
        panel::apply_result(
            &mut view,
            &ServerMechanicResponse {
                command_id: frame.command_id,
                server_sequence: frame.server_sequence,
                status: CommandStatus::Accepted,
                payload: frame.payload,
            },
        );
        let result = view.reply.as_ref().ok_or("client depot result missing")?;
        let text = panel::panel_text(&view);
        let expected_text = format!("{heading}\nResult: {outcome:?}\nRetrieved: {retrieved}\n\n");
        let holdings = format!(
            "Row 0 | Item {client_id}: depot {units}, stash 0, inbox 0; total {units}; tier {tier} x{units}"
        );
        if view.pending
            || view.feedback.is_some()
            || result.outcome != outcome
            || result.retrieved != retrieved
            || result.results.len() != usize::from(has_row)
            || text.strip_prefix(&expected_text)
                != Some(if has_row {
                    &holdings
                } else {
                    "No matching holdings on this page"
                })
        {
            return Err(format!("client depot panel mismatch: {text}").into());
        }
    }
    Ok(())
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
    expected: impl Fn(&DevClientSession) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !expected(client) {
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
    to_locker(&mut client, world).await?;
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
    let (before, _) = search_command(&mut client, wire::Request::Search { client_id }).await?;
    if before.outcome != wire::Outcome::NotFound
        || before.retrieved != 0
        || !before.results.is_empty()
    {
        return Err("depot search pre-state was not empty".into());
    }
    let original_count = carried_count(&client, item.item_definition_ref);
    let deposited_units = item.count.get();
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
    wait_carried(&mut client, |client| !carried(client, item.handle)).await?;
    let deposited_count = carried_count(&client, item.item_definition_ref);
    if deposited_count + u64::from(deposited_units) != original_count {
        return Err("deposit-inventory-count-mismatch".into());
    }

    let deposited_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(clients, 0x3c, 1).await?;
    if client.join_snapshot().game_session_id == deposited_session
        || carried_count(&client, item.item_definition_ref) != deposited_count
    {
        return Err("deposit inventory did not survive fresh admission".into());
    }
    to_locker(&mut client, world).await?;
    let previous_handle = last_carried_handle(&client);
    let (found, found_bytes) =
        search_command(&mut client, wire::Request::Search { client_id }).await?;
    let row = found.results.first().ok_or("search returned no row")?;
    let tier = row.tiers.first().ok_or("search row has no tier")?;
    if found.outcome != wire::Outcome::Found
        || found.retrieved != 0
        || row.client_id != client_id
        || found.results.len() != 1
        || row.depot != deposited_units
        || row.stash != 0
        || row.inbox != 0
        || row.tiers.len() != 1
        || row.tiers[0].count != deposited_units
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
    if detail.outcome != wire::Outcome::Detailed
        || detail.retrieved != 0
        || detail.results.as_slice() != std::slice::from_ref(row)
    {
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
    let restored_count = deposited_count + u64::from(retrieved.retrieved);
    wait_carried(&mut client, |client| {
        restored(client, item, previous_handle)
            && carried_count(client, item.item_definition_ref) == restored_count
    })
    .await?;
    let (empty, empty_bytes) = search_command(
        &mut client,
        wire::Request::Retrieve {
            client_id,
            all: true,
        },
    )
    .await?;
    if empty.outcome != wire::Outcome::NotFound
        || empty.retrieved != 0
        || !empty.results.is_empty()
        || carried_count(&client, item.item_definition_ref) != original_count
    {
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
    capture("e08-search.bin", &found_bytes.payload)?;
    capture("e08-detail.bin", &detail_bytes.payload)?;
    capture("e08-retrieve.bin", &retrieved_bytes.payload)?;
    capture("e08-empty.bin", &empty_bytes.payload)?;
    capture("e08-expected.txt", expected.as_bytes())?;
    if let Some(directory) = std::env::var_os("NODE_BOOT_E08_CAPTURE_DIR") {
        replay(
            Path::new(&directory),
            &uuid_text(client.join_snapshot().game_session_id.as_bytes()),
            client_id,
            tier_number,
            deposited_units,
            [found_bytes, detail_bytes, retrieved_bytes, empty_bytes],
        )?;
    }
    let retrieved_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0x3d, 1).await?;
    if fresh.join_snapshot().game_session_id == retrieved_session
        || carried_count(&fresh, item.item_definition_ref) != original_count
    {
        return Err("retrieved inventory did not survive fresh admission".into());
    }
    to_locker(&mut fresh, world).await?;
    let (saved, _) = search_command(&mut fresh, wire::Request::Search { client_id }).await?;
    if saved != empty {
        return Err("retrieved depot state did not survive fresh admission".into());
    }
    logout(&mut fresh).await?;
    Ok(format!(
        "item={client_id} depot={} retrieved={}",
        first.depot, retrieved.retrieved
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(_)) if std::env::var_os("NODE_BOOT_E08_CAPTURE_DIR").is_some() => evidence(
            "e2e_all scenario=E08.client status=PASS seconds=0.000 evidence=fresh-depot-search-client-replay",
        ),
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
