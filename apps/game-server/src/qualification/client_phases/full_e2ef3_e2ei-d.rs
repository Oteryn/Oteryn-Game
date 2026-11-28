//! Ordinary acquisition and item lifecycle checks for the E2EF3 item rows.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::npc_shop;
use oteryn_protocol_oteryn::world_map::MapOrigin;
use oteryn_protocol_oteryn::world_object::{self, MapUseTarget, UseIntent, UseTarget};
use oteryn_protocol_oteryn::{item_view as items, npc_service as npc, timed_item as timed};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

const I08: &str = "I08";
const I12: &str = "I12";
const I13: &str = "I13";
const I15: &str = "I15";
pub(super) const COVERED_ROWS: &[&str] = &[I08, I12, I13, I15];

struct Check {
    id: &'static str,
    action: &'static str,
    started: std::time::Instant,
}

impl Check {
    fn new(id: &'static str, action: &'static str) -> Self {
        Self {
            id,
            action,
            started: std::time::Instant::now(),
        }
    }

    fn finish(self, result: TestResult<()>) {
        let seconds = self.started.elapsed().as_secs_f64();
        let id = self.id;
        match result {
            Ok(()) => evidence(&format!(
                "e2e_all scenario={id}.full status=PASS seconds={seconds:.3}"
            )),
            Err(error) => {
                let cause =
                    verdict::classify(error.as_ref()).suffix(&format!("{id}/{}", self.action));
                evidence(&format!(
                    "e2e_all scenario={id}.full status=FAIL seconds={seconds:.3}{cause}"
                ));
                let observed = error.to_string().replace(['\r', '\n'], " ");
                evidence(&format!(
                    "E2EF3_GAP row={id} unexecuted={} observed={observed}",
                    self.action
                ));
            }
        }
    }
}

pub(super) async fn run(
    observations: TestResult<DevClientSession>,
    clients: &SeamClients<'_>,
    primal: bool,
    covet: bool,
    starter_seconds: f64,
) -> TestResult<DevClientSession> {
    for (id, used, action, route) in [
        (
            I08,
            primal,
            "obtain-primal-bag",
            "Primal Bag 39546: no carried bag observed, Hazard 12 Primal Ordeal reward held and no Plunder Patriarch spawn in the compiled town regions (reward_bag_use.rs:16)",
        ),
        (
            I13,
            covet,
            "obtain-bag-you-covet",
            "Bag You Covet 43895: no carried bag observed, Rotten Blood bosses Bakragore, Chagorz, Ichgahal, Murcion and Vemiath have no spawn in the compiled town regions and Crimson Offering requires a Bakragore kill (reward_bag_use.rs:16)",
        ),
    ] {
        match used {
            true => evidence(&format!(
                "e2e_all scenario={id}.full status=PASS seconds={starter_seconds:.3} evidence=use-consume-pinned-reward-fresh-admission-quantities"
            )),
            false => Check::new(id, action).finish(Err(route.into())),
        }
    }
    let mut mining = Check::new(I12, "obtain-pick-and-shiny-stone");
    let mut ring = Check::new(I15, "obtain-energy-ring");
    let mut client = match observations {
        Ok(client) => client,
        Err(error) => match connect_world(clients, 0x79).await {
            Ok(client) => client,
            Err(rejoin) => {
                let detail = format!("starter={error}; recovery-admission={rejoin}");
                mining.finish(Err(detail.clone().into()));
                ring.finish(Err(detail.into()));
                return Err(rejoin);
            }
        },
    };
    let setup = load_hunt_world(clients)
        .and_then(|(metadata, world)| Ok((metadata, world, provisioning::item_index()?)));
    match setup {
        Ok((metadata, world, index)) => {
            let result = refine(&mut client, clients, &mut mining, world, &metadata, &index).await;
            mining.finish(result);
            let result =
                energy_ring(&mut client, clients, &mut ring, world, &metadata, &index).await;
            ring.finish(result);
        }
        Err(error) => {
            mining.finish(Err(error.to_string().into()));
            ring.finish(Err(error));
        }
    }
    Ok(client)
}

pub(super) fn load_hunt_world(
    clients: &SeamClients<'_>,
) -> TestResult<(
    oteryn_world_bundle::bundle::Visited,
    &'static QualificationWorld,
)> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world_generation(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    Ok((metadata, world))
}

pub(super) async fn hunt_for_carried_gold(
    client: &mut DevClientSession,
    world: &QualificationWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    gold: NonZeroU32,
    minimum: u64,
) -> TestResult {
    hunt_for(client, world, spawns, 3031, |client| {
        quantities(client).is_ok_and(|stock| carried_coin_count(&stock, gold) >= minimum)
    })
    .await
}

pub(super) fn carried_coin_count(stock: &BTreeMap<u32, u64>, coin: NonZeroU32) -> u64 {
    stock.get(&coin.get()).copied().unwrap_or(0)
}

fn carried(client: &DevClientSession, reference: NonZeroU32) -> Option<items::ItemHandle> {
    client
        .inventory()?
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == reference)
        .map(|entry| entry.handle)
}

async fn rejoin(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    stage: Option<TilePos>,
) -> TestResult<()> {
    let before = client.join_snapshot().game_session_id;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    verdict::Harness::global().bounded_wait(deadline, "relog-after-combat");
    loop {
        use oteryn_protocol_oteryn::logout::LogoutResult;
        let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(client).await?;
        match result {
            Some(LogoutResult::Accepted) => break,
            Some(LogoutResult::InFight { .. }) if tokio::time::Instant::now() < deadline => {
                client.service_liveness(Duration::from_millis(250)).await?;
                client.take_events();
            }
            _ => {
                return Err(format!(
                    "logout={logout} attempts={attempts} busy_timed_out={busy_timed_out}"
                )
                .into());
            }
        }
    }
    let position = if let Some(at) = stage {
        let position = full_e2ef3_e2ei_b::position(at);
        reset_core_fixture_position(clients, position).await?;
        position
    } else {
        client.world_spatial().actor_position
    };
    *client = connect_world(clients, 0x79).await?;
    if client.world_spatial().actor_position != position {
        return Err(format!(
            "fresh admission moved character from {position:?} to {:?}",
            client.world_spatial().actor_position
        )
        .into());
    }
    if client.join_snapshot().game_session_id == before {
        return Err(format!("reused game session {before:?}").into());
    }
    Ok(())
}

pub(super) async fn buy_from_gorn(
    client: &mut DevClientSession,
    world: &BundleWorld,
    item: NonZeroU32,
    coin: NonZeroU32,
    quantity: u32,
    unit_price: u64,
) -> TestResult<()> {
    // Gorn offers the same pick (50 gp) and five-slot presents (6 gp)
    // at a reachable Thais placement.
    let shop = TilePos {
        x: 32377,
        y: 32200,
        floor: -7,
    };
    walk_near(client, world, shop, 3).await?;
    let target = e2ei_c::approach_vendor(client, world, "Gorn", |at| {
        e2ei_c::near_vendor_placement(at, shop)
    })
    .await?;
    for text in ["hi", "trade"] {
        let response = npc_shop::talk(
            client.session_mut(),
            &npc::NpcTalkIntent {
                npc_actor: target,
                text: text.into(),
            },
        )
        .await?;
        if response != npc::NpcIntentDisposition::Ok {
            return Err(format!("Gorn {text}: {response:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = npc::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("Gorn trade window: {error:?}"))?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == npc::NpcTradeSide::Buy
                && offer.item_definition_ref == item.get()
                && u64::from(offer.unit_price) == unit_price
        })
        .ok_or_else(|| format!("Gorn live window lacks definition={item} at price={unit_price}"))?;
    let mut expected = quantities(client)?;
    *expected.entry(item.get()).or_default() += u64::from(quantity) * u64::from(offer.count);
    let cost = u64::from(quantity) * unit_price;
    let before = carried_coin_count(&expected, coin);
    let after = before
        .checked_sub(cost)
        .ok_or_else(|| format!("Gorn requires {cost} carried gold; observed {before}"))?;
    if after == 0 {
        expected.remove(&coin.get());
    } else {
        expected.insert(coin.get(), after);
    }
    let catalogue_revision = window.open.ok_or("Gorn trade window closed")?.1;
    // The shop's direct trade accepts one non-stackable item per command.
    for purchased in 0..quantity {
        let response = npc_shop::buy(
            client.session_mut(),
            &npc::NpcTradeIntent {
                npc_actor: target,
                catalogue_revision,
                offer_index: u32::try_from(offer_index)?,
                side: offer.side,
                quantity: 1,
                expected_unit_price: offer.unit_price,
            },
        )
        .await?;
        if response != npc::NpcIntentDisposition::Ok {
            return Err(format!(
                "Gorn buy definition={item}: reply={response:?} purchased={purchased}/{quantity} starting_gold={before} price={unit_price}"
            ).into());
        }
    }
    wait(client, Duration::from_secs(5), |client| {
        (quantities(client).ok()? == expected).then_some(())
    })
    .await?;
    Ok(())
}

async fn walk_near(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: TilePos,
    range: u16,
) -> TestResult<()> {
    let here = tile(client.world_spatial().actor_position)?;
    let path = route_avoiding(
        world,
        here,
        |at| {
            at.floor == target.floor
                && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= range
        },
        |at| route_available(at) && at != WOLF_BLOCKED_LANDING,
    )
    .ok_or_else(|| format!("no ordinary route from {here:?} to {target:?}"))?;
    walk(client, world, &path).await
}

async fn wait<T>(
    client: &mut DevClientSession,
    limit: Duration,
    mut observe: impl FnMut(&DevClientSession) -> Option<T>,
) -> TestResult<T> {
    let deadline = tokio::time::Instant::now() + limit;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        if let Some(value) = observe(client) {
            return Ok(value);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "deadline; inventory={:?} vitals={:?}",
                client.inventory(),
                client.actor_vitals()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

pub(super) const WOLF_BLOCKED_LANDING: TilePos = TilePos {
    x: 32428,
    y: 32158,
    floor: -7,
};

async fn hunt_for(
    client: &mut DevClientSession,
    world: &QualificationWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    item: u32,
    ready: impl Fn(&DevClientSession) -> bool,
) -> TestResult<()> {
    if ready(client) {
        return Ok(());
    }
    let key = format!("oteryn:item.tibia.i{item}");
    let sources: BTreeSet<_> = spawns
        .creatures
        .iter()
        .filter(|creature| {
            world.rewards.row(&creature.key).is_ok_and(|reward| {
                reward.loot_table.entries.iter().any(|entry| {
                    entry.item.production_key == key && entry.probability_ppm != Some(0)
                })
            })
        })
        .map(|creature| creature.key.as_str())
        .collect();
    if sources.is_empty() {
        return Err(format!(
            "{key}: no compiled creature has this item in its admitted loot table"
        )
        .into());
    }
    let reference = provisioning::item_reference(&provisioning::item_index()?, item)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(deadline, "item-loot-budget");
    let hunt = e2ei_c::Hunt {
        world: &world.bundle,
        spawns,
    };
    let mut attempts = 0;
    while tokio::time::Instant::now() < deadline {
        let corpse = hunt
            .hunt_sources(client, deadline, &sources)
            .await
            .map_err(|error| {
                format!("{key}: acquisition after {attempts} kills from {sources:?}: {error}")
            })?;
        attempts += 1;
        for entry in corpse
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == reference)
        {
            let mut expected = quantities(client)?;
            *expected.entry(reference.get()).or_default() += u64::from(entry.count.get());
            let reply = client
                .move_item(&items::ItemMoveIntent {
                    source: entry.handle,
                    destination: items::ItemMoveDestination::MainBackpack,
                })
                .await?;
            if reply.outcome != items::ItemMoveOutcome::Moved {
                return Err(format!("{key}: corpse loot refused: {reply:?}").into());
            }
            wait(client, Duration::from_secs(5), |client| {
                (quantities(client).ok()? == expected).then_some(())
            })
            .await?;
            if ready(client) {
                return Ok(());
            }
        }
    }
    Err(format!("{key}: acquisition budget exhausted after {attempts} kills from {sources:?}; inventory={:?}", client.inventory()).into())
}

// The town crop contains the hunt rooms, but a previous journey can leave the
// shared fixture on a disconnected floor. Start the existing hunt budget near
// authored spawns, as for the rat and Energy Ring journeys.
const TROLL_HUNT_APPROACH: TilePos = TilePos {
    x: 32380,
    y: 32153,
    floor: -8,
};
const STONE_GOLEM_HUNT_APPROACH: TilePos = TilePos {
    x: 32719,
    y: 32520,
    floor: -10,
};

async fn refine(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    check: &mut Check,
    world: &QualificationWorld,
    metadata: &oteryn_world_bundle::bundle::Visited,
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult<()> {
    let pick = provisioning::item_reference(index, 3456)?;
    let stone = provisioning::item_reference(index, 10310)?;
    let gold = provisioning::item_reference(index, 3031)?;
    if carried(client, pick).is_none() {
        if carried_coin_count(&quantities(client)?, gold) < 50 {
            rejoin(client, clients, Some(TROLL_HUNT_APPROACH)).await?;
            hunt_for_carried_gold(client, world, &metadata.spawns, gold, 50).await?;
        }
        rejoin(client, clients, Some(TOWN)).await?;
        buy_from_gorn(client, &world.bundle, pick, gold, 1, 50).await?;
    }
    if carried(client, stone).is_none() {
        rejoin(client, clients, Some(STONE_GOLEM_HUNT_APPROACH)).await?;
        hunt_for(client, world, &metadata.spawns, 10310, |client| {
            carried(client, stone).is_some()
        })
        .await?;
        rejoin(client, clients, Some(TOWN)).await?;
    }
    let source = carried(client, pick).ok_or("pick 3456 absent after acquisition")?;
    let target = carried(client, stone).ok_or("Shiny Stone 10310 absent after acquisition")?;
    let mut expected = quantities(client)?;
    let quantity = expected
        .get_mut(&stone.get())
        .ok_or("stone quantity absent")?;
    *quantity -= 1;
    if *quantity == 0 {
        expected.remove(&stone.get());
    }
    let allowed = [3043, 3031, 3035, 3028]
        .map(|id| provisioning::item_reference(index, id))
        .into_iter()
        .collect::<TestResult<Vec<_>>>()?;
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(target)),
        use_with_position: None,
    })
    .map_err(|error| format!("pick USE intent: {error:?}"))?;
    check.action = "use-with";
    let response = client.session_mut().world_context_use(&payload).await?;
    let disposition = world_object::decode_use_result(&response.payload)
        .map_err(|error| format!("pick USE result: {error:?}"))?;
    if response.status != CommandStatus::Accepted
        || disposition != world_object::UseDisposition::Committed
    {
        return Err(format!(
            "pick 3456 USE with stone 10310: {:?}/{disposition:?}",
            response.status
        )
        .into());
    }
    check.action = "stone-consumed";
    let saved = wait(client, Duration::from_secs(5), |client| {
        let observed = quantities(client).ok()?;
        (observed.get(&stone.get()) == expected.get(&stone.get())).then_some(observed)
    })
    .await?;
    check.action = "retained-draw";
    if !allowed.into_iter().any(|reference| {
        let mut candidate = expected.clone();
        *candidate.entry(reference.get()).or_default() += 1;
        candidate == saved
    }) {
        return Err(
            format!("pick refinement before reward={expected:?} observed={saved:?}").into(),
        );
    }
    let replay = client
        .session_mut()
        .replay_acknowledged_world_use(response.command_id, &payload)
        .await?;
    if replay.status != CommandStatus::DuplicateReplay
        || replay.payload != response.payload
        || quantities(client)? != saved
    {
        return Err(format!("refinement replay changed the result or custody: {replay:?}").into());
    }
    check.action = "relogin-refinement";
    rejoin(client, clients, None).await?;
    let observed = quantities(client)?;
    if observed != saved {
        return Err(format!("refinement before={saved:?} after={observed:?}").into());
    }
    Ok(())
}

fn equipped(client: &DevClientSession, active: NonZeroU32) -> Option<items::ItemHandle> {
    client
        .inventory()?
        .equipment
        .iter()
        .find(|entry| {
            entry.slot == items::EquipmentSlot::Ring && entry.item.item_definition_ref == active
        })
        .map(|entry| entry.item.handle)
}

async fn move_ring(
    client: &mut DevClientSession,
    source: items::ItemHandle,
    destination: items::ItemMoveDestination,
) -> TestResult<u64> {
    let result = client
        .move_item(&items::ItemMoveIntent {
            source,
            destination,
        })
        .await?;
    if result.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("ring MOVE: {result:?}").into());
    }
    Ok(result.command_id)
}

async fn inspect_timer(
    client: &mut DevClientSession,
    target: items::ItemHandle,
    revision: u64,
) -> TestResult<timed::TimedItemResult> {
    let payload = timed::encode_timed_item_intent(timed::TimedItemIntent {
        action: timed::TimedAction::Inspect,
        target,
        expected_revision: revision,
        npc: None,
    })
    .map_err(|error| format!("ring timer intent: {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            timed::CAPABILITY_TIMED_ITEM_V1,
            timed::COMMAND_TYPE_TIMED_ITEM_INTENT,
            &payload,
        )
        .await?;
    let result = timed::decode_timed_item_result(&response.payload)
        .map_err(|error| format!("ring timer result: {error:?}"))?;
    if response.status != CommandStatus::Accepted
        || result.disposition != timed::TimedDisposition::Ok
    {
        return Err(format!(
            "ring timer revision {revision}: {:?}/{result:?}",
            response.status
        )
        .into());
    }
    Ok(result)
}

const DEMONA_RING_CHEST: TilePos = TilePos {
    x: 32515,
    y: 31596,
    floor: -14,
};
const DEMONA_RING_APPROACH: TilePos = TilePos {
    x: 32514,
    ..DEMONA_RING_CHEST
};
const ENERGY_FIELD_APPROACH: TilePos = TilePos {
    x: 32474,
    y: 32248,
    floor: -8,
};

async fn claim_demona_ring(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    ring: NonZeroU32,
) -> TestResult<()> {
    let mut expected = quantities(client)?;
    *expected.entry(ring.get()).or_default() += 1;
    rejoin(client, clients, Some(DEMONA_RING_APPROACH)).await?;
    let claim: TestResult = async {
        let chest = full_e2ef3_e2ei_b::visible_item(client, DEMONA_RING_CHEST, 2472).await?;
        let MapOrigin::BaseOrdinal {
            ordinal,
            object_revision,
        } = chest.origin
        else {
            return Err(format!("Demona ring chest has no base placement: {chest:?}").into());
        };
        let id = crate::map::view::placement_key(DEMONA_RING_CHEST, ordinal)
            .ok_or("Demona ring chest has no placement key")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&id.to_be_bytes());
        let payload = world_object::encode_use_intent(&world_object::WorldObjectTarget {
            placement,
            expected_revision: object_revision,
        })
        .map_err(|error| format!("Demona ring chest USE encoding: {error:?}"))?;
        let reply = client.session_mut().world_context_use(&payload).await?;
        let (disposition, gate, occurrence) =
            world_object::decode_use_reward_result(&reply.payload).map_err(|error| {
                format!("Demona ring chest USE result: {error:?}; reply={reply:?}")
            })?;
        let expects_occurrence = client
            .selected_capabilities()
            .contains(&world_object::CAPABILITY_GAMEPLAY_OCCURRENCES_V1);
        if reply.status != CommandStatus::Accepted
            || disposition != world_object::UseDisposition::Committed
            || gate.is_some()
            || occurrence.is_some() != expects_occurrence
        {
            return Err(format!("Demona ring chest USE refused: {reply:?}").into());
        }
        wait(client, Duration::from_secs(5), |client| {
            (quantities(client).ok()? == expected).then_some(())
        })
        .await?;
        Ok(())
    }
    .await;
    let returned = rejoin(client, clients, Some(TOWN)).await;
    claim.and(returned)?;
    if quantities(client)? != expected {
        return Err("Demona ring reward inventory changed after fresh admission".into());
    }
    Ok(())
}

async fn energy_ring(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    check: &mut Check,
    world: &QualificationWorld,
    metadata: &oteryn_world_bundle::bundle::Visited,
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult<()> {
    let base = provisioning::item_reference(index, 3051)?;
    let active = provisioning::item_reference(index, 3088)?;
    if carried(client, base).is_none() && equipped(client, active).is_none() {
        if world
            .bundle
            .base()
            .tile(
                DEMONA_RING_CHEST.x,
                DEMONA_RING_CHEST.y,
                DEMONA_RING_CHEST.floor,
            )
            .is_some()
        {
            claim_demona_ring(client, clients, &world.bundle, base).await?;
        } else {
            hunt_for(client, world, &metadata.spawns, 3051, |client| {
                carried(client, base).is_some()
            })
            .await?;
        }
    }
    let here = tile(client.world_spatial().actor_position)?;
    let restore = if energy_field_route(&world.bundle, here).is_none()
        && energy_field_route(&world.bundle, ENERGY_FIELD_APPROACH).is_some()
    {
        // This authored field's room has no walking connection to Thais in the crop.
        rejoin(client, clients, Some(ENERGY_FIELD_APPROACH)).await?;
        Some(here)
    } else {
        None
    };
    let lifecycle = ring_lifecycle(client, clients, check, &world.bundle, base, active).await;
    if let Some(at) = restore {
        let returned = rejoin(client, clients, Some(at)).await;
        return lifecycle.and(returned);
    }
    lifecycle
}

async fn ring_lifecycle(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    check: &mut Check,
    world: &BundleWorld,
    base: NonZeroU32,
    active: NonZeroU32,
) -> TestResult<()> {
    check.action = "equip";
    if let Some(source) = carried(client, base) {
        move_ring(
            client,
            source,
            items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring),
        )
        .await?;
    }
    let handle = wait(client, Duration::from_secs(5), |client| {
        equipped(client, active)
    })
    .await?;
    let timer = inspect_timer(client, handle, 0).await?;
    if !timer
        .remaining_ms
        .is_some_and(|ms| (1..=600_000).contains(&ms))
    {
        return Err(format!("equipped timer: {timer:?}").into());
    }

    check.action = "damage-mana-at-two-to-one";
    observe_shield(client, check, world).await?;
    check.action = "unequip-pause";
    move_ring(client, handle, items::ItemMoveDestination::MainBackpack).await?;
    let paused_handle = wait(client, Duration::from_secs(5), |client| {
        equipped(client, active)
            .is_none()
            .then(|| carried(client, base))
            .flatten()
    })
    .await?;
    let paused = inspect_timer(client, paused_handle, 0).await?;
    client.service_liveness(Duration::from_secs(2)).await?;
    let still_paused = inspect_timer(client, paused_handle, paused.revision).await?;
    if paused.remaining_ms.is_none_or(|ms| ms == 0)
        || paused.remaining_ms >= timer.remaining_ms
        || still_paused != paused
    {
        return Err(format!("unequipped timer first={paused:?} later={still_paused:?}").into());
    }

    check.action = "relogin-timer";
    rejoin(client, clients, None).await?;
    let paused_handle =
        carried(client, base).ok_or("paused Energy Ring 3051 missing after login")?;
    let restored = inspect_timer(client, paused_handle, paused.revision).await?;
    if restored != paused {
        return Err(format!("paused timer before={paused:?} restored={restored:?}").into());
    }

    check.action = "expiry";
    let previous = quantities(client)?.get(&base.get()).copied().unwrap_or(0);
    let remaining = restored
        .remaining_ms
        .ok_or("restored timer missing remaining budget")?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let earliest_activation = server_time_ms(&mut database).await?;
    let activation_session = client.join_snapshot().game_session_id;
    let activation_command = move_ring(
        client,
        paused_handle,
        items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring),
    )
    .await?;
    wait(client, Duration::from_secs(5), |client| {
        equipped(client, active)
    })
    .await?;
    let deadline =
        tokio::time::Instant::now() + Duration::from_millis(remaining) + RING_EXPIRY_GRACE;
    let latest_activation = server_time_ms(&mut database).await?;
    let retired = |client: &DevClientSession| {
        let count = quantities(client)
            .ok()?
            .get(&base.get())
            .copied()
            .unwrap_or(0);
        let ring_slot_empty = client
            .inventory()?
            .equipment
            .iter()
            .all(|entry| entry.slot != items::EquipmentSlot::Ring);
        (ring_slot_empty && count + 1 == previous).then_some(())
    };
    wait(
        client,
        deadline.saturating_duration_since(tokio::time::Instant::now()),
        retired,
    )
    .await?;
    let retirement_sequence = client.last_server_sequence();
    let retired_at: Option<i64> = sqlx::query_scalar(
        "SELECT w.committed_at FROM game_character_equipment_receipts r
         JOIN game_item_timed_state_writes w USING (item_instance_id)
         JOIN game_item_instances i USING (item_instance_id)
         WHERE r.game_session_id=encode($1,'hex')::uuid AND r.command_id=$2::text::numeric
         AND r.to_slot IS NOT NULL AND w.holder_character_id=r.character_id AND w.cause=2
         AND w.definition_before_production_key='oteryn:item.tibia.i3051'
         AND i.lifecycle=2 AND i.last_transaction_id=w.transaction_id",
    )
    .bind(activation_session.as_bytes().as_slice())
    .bind(activation_command.to_string())
    .fetch_optional(&mut database)
    .await?;
    database.close().await?;
    check.action = "relogin-timer";
    rejoin(client, clients, None).await?;
    if retired(client).is_none() {
        return Err(format!(
            "expired ring reappeared on fresh admission; inventory={:?}",
            client.inventory()
        )
        .into());
    }
    verify_ring_expiry_timing(
        &mut check.action,
        remaining,
        retirement_sequence,
        retired_at.map(|at| (earliest_activation, latest_activation, at)),
    )
}

const RING_EXPIRY_GRACE: Duration = Duration::from_secs(5);

async fn server_time_ms(database: &mut sqlx::PgConnection) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint")
            .fetch_one(database)
            .await?,
    )
}

fn verify_ring_expiry_timing(
    action: &mut &'static str,
    remaining_ms: u64,
    retirement_sequence: u64,
    timing: Option<(i64, i64, i64)>,
) -> TestResult {
    *action = "verify-server-ring-expiry-timing";
    let Some((earliest_activation, latest_activation, retired_at)) = timing else {
        return Err(format!("SERVER-GAP missing server timestamps for Energy Ring resumed activation and retirement; restored budget={remaining_ms}ms; retirement-observed-by-sequence={retirement_sequence}; owner clock at timed_item_clock.rs:375; timer reply at timed_item.rs:42").into());
    };
    let remaining = i64::try_from(remaining_ms)?;
    let grace = i64::try_from(RING_EXPIRY_GRACE.as_millis())?;
    if earliest_activation > latest_activation
        || earliest_activation
            .checked_add(remaining)
            .is_none_or(|earliest| retired_at < earliest)
        || latest_activation
            .checked_add(remaining)
            .and_then(|latest| latest.checked_add(grace))
            .is_none_or(|latest| retired_at > latest)
    {
        return Err(format!("Energy Ring retirement {retired_at} outside resumed activation window [{earliest_activation}, {latest_activation}] with {remaining_ms}ms remaining; retirement-observed-by-sequence={retirement_sequence}").into());
    }
    Ok(())
}

fn energy_field_route(world: &BundleWorld, from: TilePos) -> Option<Vec<StepDirection>> {
    let field = |at: TilePos| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|cell| {
            !cell.protection_zone()
                && (0..cell.depths().iter().filter(|depth| **depth == 0).count())
                    .filter_map(|ordinal| u8::try_from(ordinal).ok())
                    .any(|ordinal| cell.appearance(ordinal) == Some(2126))
        })
    };
    route_avoiding(world, from, field, |at| at != WOLF_BLOCKED_LANDING)
}

async fn observe_shield(
    client: &mut DevClientSession,
    check: &mut Check,
    world: &BundleWorld,
) -> TestResult<()> {
    // Native 2126 applies one 100-point energy hit per entry, with no residual tick.
    let here = tile(client.world_spatial().actor_position)?;
    let mut path = energy_field_route(world, here).ok_or_else(|| {
        format!("native Energy Field 2126 has no walk route from {here:?} avoiding {WOLF_BLOCKED_LANDING:?}")
    })?;
    let last = path.pop().ok_or("already standing on Energy Field 2126")?;
    let retreat = match last {
        StepDirection::North => StepDirection::South,
        StepDirection::South => StepDirection::North,
        StepDirection::East => StepDirection::West,
        StepDirection::West => StepDirection::East,
        _ => return Err("energy field route ended in a diagonal step".into()),
    };
    walk(client, world, &path).await?;
    for _ in 0..10 {
        let before = *client
            .actor_vitals()
            .ok_or("energy field approach did not project actor vitals")?;
        let absorbed = 100.min(before.mana / 2);
        let mana = before.mana - 2 * absorbed;
        let health = before.health.saturating_sub(100 - absorbed);
        if absorbed == 0 || health == 0 {
            return Err(format!(
                "Energy Field 2126 cannot safely observe shield: vitals={before:?}"
            )
            .into());
        }
        walk(client, world, &[last]).await?;
        let after = wait(client, Duration::from_secs(5), |client| {
            client
                .actor_vitals()
                .copied()
                .filter(|after| after.health < before.health || after.mana < before.mana)
        })
        .await?;
        if after.mana != mana {
            return Err(format!("Energy Field 2126 damage=100 expected mana={mana} before={before:?} after={after:?}").into());
        }
        check.action = "health-spill";
        if after.health != health {
            return Err(format!("Energy Field 2126 damage=100 expected health={health} before={before:?} after={after:?}").into());
        }
        walk(client, world, &[retreat]).await?;
        if health < before.health {
            return Ok(());
        }
    }
    Err(format!(
        "Energy Field 2126 did not spill into health; last vitals={:?}",
        client.actor_vitals()
    )
    .into())
}

#[test]
fn ring_expiry_requires_server_timestamps() {
    let mut action = "relogin-timer";
    let result = verify_ring_expiry_timing(&mut action, 600, 42, None);
    assert_eq!(action, "verify-server-ring-expiry-timing");
    assert!(result.is_err_and(|error| {
        let error = error.to_string();
        error.contains("missing server timestamps")
            && error.contains("restored budget=600ms")
            && error.contains("retirement-observed-by-sequence=42")
    }));
}

#[test]
fn ring_expiry_requires_the_restored_budget_within_the_activation_window() -> TestResult {
    let mut action = "relogin-timer";
    verify_ring_expiry_timing(&mut action, 600, 42, Some((800, 825, 1425)))?;
    for timing in [(800, 825, 1399), (800, 825, 6426), (825, 800, 1425)] {
        assert!(verify_ring_expiry_timing(&mut action, 600, 42, Some(timing)).is_err());
    }
    Ok(())
}

#[test]
fn lubo_funding_counts_only_carried_gold_coins() -> TestResult {
    let gold = NonZeroU32::new(3031).ok_or("gold definition")?;
    assert_eq!(carried_coin_count(&BTreeMap::new(), gold), 0);
    assert_eq!(carried_coin_count(&BTreeMap::from([(3035, 38)]), gold), 0);
    assert_eq!(carried_coin_count(&BTreeMap::from([(3031, 38)]), gold), 38);
    Ok(())
}
