//! N03 client replay capture: deposit a carried item at an authored depot locker, then capture the
//! real Depot Search holdings list (every page up to the item) and the RETRIEVE(one) reply. Client
//! owner: `activate_shared` in `apps/client/src/features/0153-depot-search/native.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::{CommandStatus, depot, depot_search as wire};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
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
    carried_from(client.inventory(), handle)
}

fn carried_from(input: Option<&oteryn_dev_client::CharacterInventory>, handle: ItemHandle) -> bool {
    input.is_some_and(|inventory| {
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(150);
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
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
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

        // List the holdings page by page, as the client does (first page, then next page after the
        // last row), until the deposited item is listed.
        let mut pages = Vec::new();
        let mut after = 0;
        let (listed, row) = loop {
            let (page, page_bytes) =
                search_command(&mut client, wire::Request::Open { after }).await?;
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
        if row.depot == 0 {
            return Err(format!("listed row has no depot count: {row:?}").into());
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
            capture(&format!("n03-list-{index}.bin"), bytes)?;
        }
        capture("n03-retrieve.bin", &retrieved_bytes)?;
        capture("n03-expected.txt", expected.as_bytes())?;
        // Return whatever stays in the depot so later rows find the carried item again.
        match search_command(
            &mut client,
            wire::Request::Retrieve {
                client_id,
                all: true,
            },
        )
        .await
        {
            Ok((rest, _))
                if matches!(
                    rest.outcome,
                    wire::Outcome::Retrieved | wire::Outcome::NotFound
                ) => {}
            other => evidence(&format!("N03_CLEANUP_UNCONFIRMED {other:?}").replace(' ', "_")),
        }

        Ok(format!(
            "item={client_id} pages={} depot={} retrieved={}",
            pages.len(),
            row.depot,
            retrieved.retrieved
        ))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_projection_cannot_supply_evidence() {
        assert!(!carried_from(None, std::num::NonZeroU64::MIN));
    }
}
