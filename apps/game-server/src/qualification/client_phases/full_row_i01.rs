//! I01: containers, nesting and stack quantities through the native client's own move path.
//! Buys a bag (2853) from an NPC seller with the character's own gold, opens it through the
//! client-runtime container adapter, then splits one coin off the backpack gold stack into the
//! nested bag with the client-ui drag/count prompt (`Movement::begin`, `drop_on`, `confirm`)
//! and `client_runtime::inventory::send`. The reply and the domain-4/14 deltas are applied by
//! `oteryn_session::Session`; the row asserts on that client state, re-reads both quantities
//! after logout and a fresh admission, then merges the coin back into the backpack stack.
//! Filling a container to capacity is not driven here (it needs `capacity` distinct items).
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{container_tree, inventory};
use oteryn_dev_client::client_ui::inventory::movement::{DragItem, Movement};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I01.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut values = BTreeMap::new();
    for item in &client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
    {
        *values.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(values)
}

fn quantity(client: &DevClientSession, reference: NonZeroU32) -> TestResult<u64> {
    Ok(*quantities(client)?.get(&reference.get()).unwrap_or(&0))
}

async fn wait_quantity(
    client: &mut DevClientSession,
    reference: NonZeroU32,
    expected: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        client.take_events();
        if quantity(client, reference)? == expected {
            return Ok(client.last_server_sequence());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "inventory delta timeout definition={reference} expected={expected} observed={}",
                quantity(client, reference)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

/// The quantity of `reference` the client's open view of `container` shows, `None` while the
/// session holds no view of it.
fn nested(
    client: &mut DevClientSession,
    container: oteryn_dev_client::ItemHandle,
    reference: NonZeroU32,
) -> Option<(u8, u64)> {
    let views = container_tree::initial(client.session_mut())?;
    let view = views
        .views
        .iter()
        .find(|view| view.container_handle == container)?;
    Some((
        view.capacity,
        view.entries
            .iter()
            .filter(|entry| entry.item_definition_ref == reference)
            .map(|entry| u64::from(entry.count.get()))
            .sum(),
    ))
}

async fn wait_nested(
    client: &mut DevClientSession,
    container: oteryn_dev_client::ItemHandle,
    reference: NonZeroU32,
    expected: u64,
    step: &str,
) -> TestResult<u8> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        client.take_events();
        if let Some((capacity, count)) = nested(client, container, reference)
            && count == expected
        {
            return Ok(capacity);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step={step} observed=view:{:?};needed={expected}",
                nested(client, container, reference)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    id: u32,
    amount: u32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, id)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let key = format!("oteryn:item.tibia.i{id}");
    let sellers: BTreeSet<_> = trades["records"]
        .as_array()
        .ok_or("trade records absent")?
        .iter()
        .filter_map(|record| {
            let declaration = &record["declaration"];
            declaration["offers"]
                .as_array()?
                .iter()
                .any(|offer| offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key)
                .then(|| declaration["identity"]["key"].as_str())
                .flatten()?
                .strip_prefix("oteryn:service.trade.")
                .map(|name| format!("oteryn:npc.{name}"))
        })
        .collect();
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let here = tile(client.world_spatial().actor_position)?;
    let (npc, path) = metadata
        .npcs
        .npcs
        .iter()
        .filter(|npc| sellers.contains(&npc.key))
        .filter_map(|npc| {
            route(world, here, |at| {
                npc.placements
                    .iter()
                    .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
            })
            .map(|path| (npc, path))
        })
        .min_by_key(|(_, path)| path.len())
        .ok_or_else(|| format!("no walk route to seller for item {id}; sellers={sellers:?}"))?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        entity.position.x == i32::from(p.x)
                            && entity.position.y == i32::from(p.y)
                            && entity.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or("seller not visible after walk")?;
    for text in ["hi", "trade"] {
        let reply = runtime::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != wire::NpcIntentDisposition::Ok {
            return Err(format!("NPC {text} reply={reply:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("trade window decode: {error:?}"))?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()
        })
        .ok_or("live seller omitted requested offer")?;
    let before = quantity(client, reference)?;
    client.take_events();
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade window closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: amount,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "buy item={id} reply={reply:?} price={} quantity={amount}",
            offer.unit_price
        )
        .into());
    }
    let expected = before + u64::from(offer.count) * u64::from(amount);
    wait_quantity(client, reference, expected).await
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

async fn open_world(clients: &SeamClients<'_>) -> TestResult<&'static BundleWorld> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}

fn reference(id: u32) -> TestResult<NonZeroU32> {
    let index = provisioning::item_index()?;
    provisioning::item_reference(&index, id)
}

fn backpack_entry(
    client: &DevClientSession,
    definition: NonZeroU32,
) -> Option<oteryn_dev_client::ItemEntry> {
    client
        .inventory()?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == definition)
        .max_by_key(|item| item.count)
        .copied()
}

/// Opens `bag` through the client-runtime adapter and returns its view's capacity.
async fn open(
    client: &mut DevClientSession,
    bag: oteryn_dev_client::ItemHandle,
    coin: NonZeroU32,
    expected: u64,
    step: &str,
) -> TestResult<u8> {
    client.take_events();
    let outcome = container_tree::send(
        client.session_mut(),
        container_tree::ContainerViewIntent::Open {
            handle: bag,
            replace_view: None,
        },
    )
    .await?;
    if outcome != Some(container_tree::ContainerViewOutcome::Opened) {
        return Err(format!(
            "step={step} observed={};needed=Opened",
            token(container_tree::feedback(outcome))
        )
        .into());
    }
    wait_nested(client, bag, coin, expected, step).await
}

/// One native drag-and-drop: `begin`, `drop_on` with the count prompt the client shows for a
/// stack, `confirm(count)` when prompted, then `client_runtime::inventory::send`.
async fn drag(
    client: &mut DevClientSession,
    item: oteryn_dev_client::ItemEntry,
    destination: inventory::ItemMoveDestination,
    count: Option<u32>,
    step: &str,
) -> TestResult {
    let mut movement = Movement::default();
    if !movement.begin(DragItem {
        equipped: false,
        handle: item.handle,
        count: item.count.get(),
    }) {
        return Err(format!("step={step} observed=drag-refused-by-client-state").into());
    }
    let counted = client.selected_capabilities().contains(&61);
    let request = match (movement.drop_on(Some(destination), counted), count) {
        (Some(request), None) => request,
        (None, Some(count)) if movement.prompt.is_some() => movement
            .confirm(count)
            .ok_or_else(|| format!("step={step} observed=count-prompt:{:?}", movement.feedback))?,
        (request, _) => {
            return Err(format!(
                "step={step} observed=request:{request:?};prompt:{:?};counted:{counted}",
                movement.prompt
            )
            .into());
        }
    };
    client.take_events();
    let response = inventory::send(client.session_mut(), request).await?;
    let text = inventory::feedback(response);
    if response.map(|response| response.outcome) != Some(inventory::ItemMoveOutcome::Moved) {
        return Err(format!("step={step} observed={};needed=Item-moved", token(text)).into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let world = open_world(clients).await?;
    let coin = reference(3031)?;
    let bag = reference(2853)?;
    let mut client = connect_world(clients, 0x0e).await?;
    for capability in [4, 14, 61] {
        if !client.selected_capabilities().contains(&capability) {
            logout(&mut client).await?;
            return Err(format!(
                "step=admission observed=capabilities:{:?};needed:capability-{capability}",
                client.selected_capabilities()
            )
            .into());
        }
    }
    acquire(&mut client, world, 2853, 1)
        .await
        .map_err(|error| {
            format!("step=buy-2853 observed={error};needed:player-gold-and-reachable-seller")
        })?;
    let total = quantity(&client, coin)?;
    let stack = backpack_entry(&client, coin)
        .filter(|entry| entry.count.get() >= 2)
        .ok_or_else(|| {
            format!("step=precondition observed=backpack-gold:{total};needed:a-gold-stack-of-2")
        })?;
    let bag_handle = backpack_entry(&client, bag)
        .map(|item| item.handle)
        .ok_or("step=bag observed=no-bag-handle")?;
    let capacity = open(&mut client, bag_handle, coin, 0, "open-bag").await?;
    if capacity == 0 {
        return Err("step=open-bag observed=capacity:0".into());
    }
    drag(
        &mut client,
        stack,
        inventory::ItemMoveDestination::Container(bag_handle),
        Some(1),
        "split-into-bag",
    )
    .await?;
    wait_quantity(&mut client, coin, total - 1).await?;
    wait_nested(&mut client, bag_handle, coin, 1, "split-into-bag").await?;
    logout(&mut client).await?;
    let mut client = connect_world(clients, 0x0e).await?;
    if quantity(&client, coin)? != total - 1 {
        return Err(format!(
            "step=fresh-admission observed=backpack-gold:{};needed={}",
            quantity(&client, coin)?,
            total - 1
        )
        .into());
    }
    let bag_handle = backpack_entry(&client, bag)
        .map(|item| item.handle)
        .ok_or("step=fresh-admission observed=bag-lost")?;
    open(&mut client, bag_handle, coin, 1, "fresh-admission-bag").await?;
    let split = container_tree::initial(client.session_mut())
        .and_then(|views| {
            views
                .views
                .into_iter()
                .find(|view| view.container_handle == bag_handle)?
                .entries
                .into_iter()
                .find(|entry| entry.item_definition_ref == coin)
        })
        .ok_or("step=fresh-admission observed=nested-coin-absent")?;
    drag(
        &mut client,
        split,
        inventory::ItemMoveDestination::MainBackpack,
        None,
        "merge-back",
    )
    .await?;
    wait_quantity(&mut client, coin, total).await?;
    wait_nested(&mut client, bag_handle, coin, 0, "merge-back").await?;
    logout(&mut client).await?;
    Ok(format!(
        "bag-capacity-{capacity};split-1-of-{total}-into-nested-bag;durable-after-fresh-admission;merged-back"
    ))
}
