//! I05 full-row journey: general timed items, equipment charges and transforms.
//!
//! Concrete player scenario, derived from matrix row I05, the Crystal re-audit row,
//! and `timed_item_clock` (the live server owner):
//! 1. Admit the node-boot character and send TIMED_ITEM Inspect for an unknown
//!    server handle; expect an accepted command frame containing a Rejected result
//!    and no inventory delta (the refusal path must not mutate custody).
//! 2. Earn 5,000 gold through ordinary rat combat/corpse looting, walk to a compiled
//!    world NPC who sells energy ring 3051, say `hi` and `trade`, buy one, and require
//!    the Ok trade reply plus the inventory delta that adds it.
//! 3. Send ITEM_MOVE for that carried ring into the ring equipment slot; expect
//!    Moved plus an inventory delta whose equipped view is transformed to active
//!    energy ring 3088.
//! 4. Send TIMED_ITEM Inspect for the equipped handle; expect Ok, revision zero and
//!    a positive remaining budget no greater than 600,000 ms. Send another Inspect
//!    with a deliberately wrong expected revision; expect Stale and no equipment
//!    delta, then inspect with the reported revision and expect Ok.
//! 5. Log out, obtain a fresh admission, and re-check durable equipped custody: the
//!    active 3088 view remains in the ring slot and its inspected remaining budget
//!    is positive and no greater than the pre-logout budget.
use super::*;
use oteryn_dev_client::client_runtime::npc_shop as shop_runtime;
use oteryn_protocol_oteryn::{item_view as items, npc_service as npc, timed_item as timed};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

const ENERGY_RING: u32 = 3051;
const ACTIVE_ENERGY_RING: u32 = 3088;
const PRICE: u64 = 5_000;

struct Journey {
    step: usize,
    started: std::time::Instant,
}

impl Journey {
    fn new() -> Self {
        Self { step: 0, started: std::time::Instant::now() }
    }
    fn passed(&mut self, step: usize, observation: &str) {
        self.step = step;
        evidence(&format!("I05_CHECK step={step} PASS observed={observation}"));
    }
    fn finish(self, result: TestResult) {
        if let Err(error) = &result {
            evidence(&format!("I05_GAP step={} observed={error}", self.step + 1));
        }
        evidence(&format!(
            "e2e_all scenario=I05.full status={} seconds={:.3}",
            if result.is_ok() && self.step == 5 { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Journey::new();
    let result = verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("journey-timeout:no-server-reply-within-420s".into()),
    });
}

async fn timed_command(
    client: &mut oteryn_dev_client::DevClientSession,
    action: timed::TimedAction,
    target: items::ItemHandle,
    expected_revision: u64,
) -> TestResult<timed::TimedItemResult> {
    let payload = timed::encode_timed_item_intent(timed::TimedItemIntent {
        action,
        target,
        expected_revision,
        npc: None,
    })
    .map_err(|error| format!("timed-intent-encode:{error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            timed::CAPABILITY_TIMED_ITEM_V1,
            timed::COMMAND_TYPE_TIMED_ITEM_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("timed-command-status:{:?}", reply.status).into());
    }
    timed::decode_timed_item_result(&reply.payload)
        .map_err(|error| format!("timed-result-decode:{error:?}").into())
}

fn reference(id: u32) -> TestResult<NonZeroU32> {
    let index = provisioning::item_index()?;
    provisioning::item_reference(&index, id)
}

fn carried_gold(client: &oteryn_dev_client::DevClientSession) -> TestResult<u64> {
    let coins = [(reference(3031)?, 1), (reference(3035)?, 100), (reference(3043)?, 10_000)];
    Ok(client.inventory().ok_or("gold-inventory-missing")?.entries.iter().map(|entry| {
        coins.iter().find(|(key, _)| *key == entry.item_definition_ref)
            .map_or(0, |(_, value)| u64::from(entry.count.get()) * value)
    }).sum())
}

async fn earn_gold(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let starting_gold = carried_gold(client)?;
    if starting_gold >= PRICE {
        return Err(format!(
            "step-2-combat-required-but-skipped:starting-gold={starting_gold}:required={PRICE}"
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while carried_gold(client)? < PRICE {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("combat-loot-budget-exhausted:gold={}:required={PRICE}", carried_gold(client)?).into());
        }
        travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await
            .map_err(|error| format!("rat-walk:{error}"))?;
        client_hunt::hunt(client).await.map_err(|error| format!("rat-combat-loot:{error}"))?;
    }
    Ok(())
}

fn item_handle(client: &oteryn_dev_client::DevClientSession, key: NonZeroU32) -> TestResult<items::ItemHandle> {
    client.inventory().and_then(|view| view.entries.iter().find(|entry| entry.item_definition_ref == key))
        .map(|entry| entry.handle).ok_or_else(|| format!("inventory-item-absent:{key}").into())
}

fn item_count(client: &oteryn_dev_client::DevClientSession, key: NonZeroU32) -> TestResult<u64> {
    Ok(client.inventory().ok_or("item-count-inventory-missing")?.entries.iter()
        .filter(|entry| entry.item_definition_ref == key)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

async fn wait_for_item(
    client: &mut oteryn_dev_client::DevClientSession,
    key: NonZeroU32,
    previous_count: u64,
) -> TestResult<items::ItemHandle> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if item_count(client, key)? > previous_count {
            return item_handle(client, key);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "purchase-inventory-delta-timeout:item={key}:previous-count={previous_count}:observed-count={}",
                item_count(client, key)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn buy_energy_ring(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<items::ItemHandle> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalogue: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let key = format!("oteryn:item.tibia.i{ENERGY_RING}");
    let sellers: BTreeSet<String> = catalogue["records"].as_array().ok_or("trade-records-missing")?
        .iter().filter_map(|record| {
            let declaration = &record["declaration"];
            declaration["offers"].as_array()?.iter().any(|offer| {
                offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key
            }).then(|| declaration["identity"]["key"].as_str()).flatten()
                .and_then(|service| service.strip_prefix("oteryn:service.trade."))
                .map(|name| format!("oteryn:npc.{name}"))
        }).collect();
    let here = tile(client.world_spatial().actor_position)?;
    let (seller, path) = npcs.npcs.iter().filter(|entry| sellers.contains(&entry.key))
        .filter_map(|entry| route(world, here, |at| entry.placements.iter().any(|place| {
            place.floor == at.floor && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
        })).map(|path| (entry, path)))
        .min_by_key(|(_, path)| path.len())
        .ok_or_else(|| format!("energy-ring-seller-unreachable:sellers={sellers:?}"))?;
    walk(client, world, &path).await?;
    let actor = client.world_entities().and_then(|view| view.iter().find(|entry| {
        entry.kind == oteryn_dev_client::EntityKind::Npc && seller.placements.iter().any(|place| {
            entry.position.x == i32::from(place.x) && entry.position.y == i32::from(place.y)
                && entry.position.floor == -i16::from(place.floor)
        })
    })).map(|entry| entry.entity).ok_or("energy-ring-seller-actor-missing")?;
    for text in ["hi", "trade"] {
        let reply = shop_runtime::talk(client.session_mut(), &npc::NpcTalkIntent { npc_actor: actor, text: text.into() }).await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!("npc-talk-refused:text={text}:reply={reply:?}").into());
        }
    }
    let delta = client.session_mut().read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1).await?;
    let window = npc::decode_npc_trade_window(&delta.payload).map_err(|error| format!("trade-window-decode:{error:?}"))?;
    let wanted = reference(ENERGY_RING)?;
    let (offer_index, offer) = window.offers.iter().enumerate().find(|(_, offer)| {
        offer.side == npc::NpcTradeSide::Buy && offer.item_definition_ref == wanted.get()
    }).ok_or("energy-ring-live-offer-missing")?;
    let previous_count = item_count(client, wanted)?;
    let reply = shop_runtime::buy(client.session_mut(), &npc::NpcTradeIntent {
        npc_actor: actor,
        catalogue_revision: window.open.ok_or("energy-ring-trade-window-closed")?.1,
        offer_index: u32::try_from(offer_index)?,
        side: offer.side,
        quantity: 1,
        expected_unit_price: offer.unit_price,
    }).await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("energy-ring-buy-refused:reply={reply:?}:gold={}:price={}", carried_gold(client)?, offer.unit_price).into());
    }
    wait_for_item(client, wanted, previous_count).await
}

fn equipped_ring(client: &oteryn_dev_client::DevClientSession, active: NonZeroU32) -> TestResult<items::ItemHandle> {
    client.inventory().and_then(|view| view.equipment.iter().find(|entry| {
        entry.slot == items::EquipmentSlot::Ring && entry.item.item_definition_ref == active
    })).map(|entry| entry.item.handle).ok_or("active-energy-ring-equipment-delta-missing".into())
}

async fn journey(row: &mut Journey, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin: serde_json::Value = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = connect_world(clients, 0xb0).await?;
    let result: TestResult = async {
        if !client.selected_capabilities().contains(&timed::CAPABILITY_TIMED_ITEM_V1) {
            return Err("timed-item-capability-not-offered".into());
        }
        let before = client.inventory().cloned().ok_or("refusal-inventory-missing")?;
        let unknown = items::ItemHandle::new(u64::MAX).ok_or("invalid-test-handle")?;
        let refused = timed_command(&mut client, timed::TimedAction::Inspect, unknown, 0).await?;
        if refused != timed::TimedItemResult::refused(timed::TimedDisposition::Rejected)
            || client.inventory() != Some(&before) {
            return Err(format!("unknown-handle-not-rollback-safe:reply={refused:?}").into());
        }
        row.passed(1, "timed-rejected+inventory-unchanged");

        earn_gold(&mut client, world, &metadata.spawns).await?;
        let carried = buy_energy_ring(&mut client, world, &metadata.npcs).await?;
        row.passed(2, "combat-loot+npc-talk+trade-ok+inventory-delta");

        client.take_events();
        let moved = client.move_item(&items::ItemMoveIntent {
            source: carried,
            destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring),
        }).await?;
        if moved.outcome != items::ItemMoveOutcome::Moved {
            return Err(format!("energy-ring-equip-refused:reply={moved:?}").into());
        }
        let active = reference(ACTIVE_ENERGY_RING)?;
        let handle = equipped_ring(&client, active)?;
        row.passed(3, "item-move-moved+3051-to-3088-equipment-delta");

        let inspected = timed_command(&mut client, timed::TimedAction::Inspect, handle, 0).await?;
        let remaining = inspected.remaining_ms.ok_or("inspect-missing-remaining-ms")?;
        if inspected.disposition != timed::TimedDisposition::Ok
            || inspected.revision != 0
            || remaining == 0
            || remaining > 600_000
        {
            return Err(format!("equipped-inspect-invalid:reply={inspected:?}").into());
        }
        let equipment = client.inventory().map(|view| view.equipment.clone()).ok_or("stale-equipment-missing")?;
        let stale = timed_command(&mut client, timed::TimedAction::Inspect, handle, inspected.revision.saturating_add(1)).await?;
        if stale.disposition != timed::TimedDisposition::Stale
            || client.inventory().is_none_or(|view| view.equipment != equipment) {
            return Err(format!("wrong-revision-not-stale-or-mutated:reply={stale:?}").into());
        }
        let current = timed_command(&mut client, timed::TimedAction::Inspect, handle, inspected.revision).await?;
        if current.disposition != timed::TimedDisposition::Ok { return Err(format!("current-revision-inspect-refused:reply={current:?}").into()); }
        row.passed(4, "inspect-ok+revision-zero+bounded-budget+stale-refusal+no-delta+current-revision-ok");

        let old_session = client.join_snapshot().game_session_id;
        if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err("logout-refused-before-persistence-check".into());
        }
        client = connect_world(clients, 0xb1).await?;
        if client.join_snapshot().game_session_id == old_session { return Err("fresh-admission-reused-session".into()); }
        let durable_handle = equipped_ring(&client, active)?;
        let durable = timed_command(&mut client, timed::TimedAction::Inspect, durable_handle, current.revision).await?;
        if durable.disposition != timed::TimedDisposition::Ok
            || durable.remaining_ms.is_none_or(|value| value == 0 || value > remaining) {
            return Err(format!("fresh-admission-timer-not-durable:before={remaining}:reply={durable:?}").into());
        }
        row.passed(5, "fresh-session+active-transform+nonincreasing-timer");
        Ok(())
    }.await;
    let release = async {
        if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err("final-logout-refused".into());
        }
        Ok(())
    }.await;
    result.and(release)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn required_item_ids_and_price_are_fixed() {
        assert_eq!((ENERGY_RING, ACTIVE_ENERGY_RING, PRICE), (3051, 3088, 5_000));
    }
}
