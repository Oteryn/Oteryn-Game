//! C02 full-row journey, derived from the matrix, re-audit, and `attack/ranged.rs` owner.
//!
//! 1. Admit the dedicated character and load its qualification purchase funds,
//!    walk to compiled-world sellers, say `hi` and `trade`, and buy a bow, ten arrows, a Wand of
//!    Vortex; require every accepted trade reply and exact inventory delta.
//! 2. Equip the bow and arrows with ITEM_MOVE, send ATTACK against a live rat, and require an Ok
//!    attack reply, an exact one-arrow inventory decrement, and a damage presentation attributed
//!    from this character to that rat. Continue firing until all arrows are consumed, then send
//!    ATTACK again and require the Ok intent reply but no damage presentation, mana debit, or
//!    inventory change during a complete attack interval (the server's no-ammunition refusal).
//! 3. Equip the Wand of Vortex, send ATTACK against a live rat, and require the Ok reply, an exact
//!    two-mana debit, attributed energy-coloured damage without inventory consumption, and kill XP.
//! 4. Buy a Snakebite Rod on a separate Druid, equip it, and send ATTACK. Require an exact
//!    one-mana debit, attributed earth-coloured damage without inventory consumption, and kill XP.
//! 5. Check each Character on fresh admission for new session identity and durable custody,
//!    ammunition, mana, and kill XP.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::{
    attack::AttackIntentDisposition,
    item_view::{self as items, EquipmentSlot},
    npc_service as npc,
    presentation::{PresentationColor, PresentationEventKind},
};
use std::{collections::BTreeSet, num::NonZeroU32};

const BOW: u32 = 3350;
const ARROW: u32 = 3447;
const WAND: u32 = 3074;
const ROD: u32 = 3066;
const WAND_MANA: u32 = 2;
const ROD_MANA: u32 = 1;

// These packet callers use the local signer in world::connect_character.
// Gameplay checks can still run, but no packet can attest to the Platform issuer.
pub(super) const PLATFORM_ADMISSION_GAP: &str =
    "need=platform-issued-admission-grant;observed=qualification-local-signer";

struct Row {
    started: std::time::Instant,
    step: usize,
}
fn durable_rewards_match(earned_xp: u64, admitted_xp: u64) -> bool {
    earned_xp == admitted_xp
}

impl Row {
    fn pass(&mut self, step: usize, observed: impl std::fmt::Debug) {
        self.step = step;
        evidence(&format!("C02_CHECK step={step} PASS observed={observed:?}"));
    }
    fn finish(self, result: TestResult) {
        let result = result.and_then(|()| {
            if self.step == 5 {
                Ok(())
            } else {
                Err("missing-required-ranged-step".into())
            }
        });
        if let Err(error) = &result {
            evidence(&format!(
                "C02_GAP step={} observed={error} need=accepted-native-reply-and-required-exact-deltas",
                self.step + 1
            ));
        }
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C02.full status={} seconds={:.3}{}",
            if result.is_ok() && self.step == 5 {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
            if result.is_ok() {
                ""
            } else {
                " evidence=required-step-failed-see-C02_GAP"
            }
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row {
        started: std::time::Instant::now(),
        step: 0,
    };
    let result = verdict::indexed_timeout(
        Duration::from_secs(420),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err(format!("timeout-after-step-{}", row.step).into()),
    };
    row.finish(result);
}

pub(super) fn reference(id: u32) -> TestResult<NonZeroU32> {
    provisioning::item_reference(&provisioning::item_index()?, id)
}
pub(super) fn count(client: &DevClientSession, key: NonZeroU32) -> TestResult<u64> {
    Ok(inventory_count(
        client.inventory().ok_or("inventory-frame-absent")?,
        key,
    ))
}

fn inventory_count(inventory: &items::CharacterInventory, key: NonZeroU32) -> u64 {
    inventory
        .entries
        .iter()
        .chain(inventory.equipment.iter().map(|e| &e.item))
        .filter(|e| e.item_definition_ref == key)
        .map(|e| u64::from(e.count.get()))
        .sum()
}
fn carried(client: &DevClientSession, key: NonZeroU32) -> TestResult<items::ItemHandle> {
    client
        .inventory()
        .and_then(|v| v.entries.iter().find(|e| e.item_definition_ref == key))
        .map(|e| e.handle)
        .ok_or_else(|| format!("carried-item-absent:{key}").into())
}
async fn wait_count(
    client: &mut DevClientSession,
    key: NonZeroU32,
    wanted: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event
                && count(client, key)? == wanted
            {
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "inventory-delta-timeout:item={key}:observed={}:wanted={wanted}",
                count(client, key)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

fn coin_references() -> TestResult<[(NonZeroU32, u64); 3]> {
    let index = provisioning::item_index()?;
    Ok([
        (provisioning::item_reference(&index, 3031)?, 1),
        (provisioning::item_reference(&index, 3035)?, 100),
        (provisioning::item_reference(&index, 3043)?, 10_000),
    ])
}

fn carried_gold(client: &DevClientSession, coins: &[(NonZeroU32, u64); 3]) -> TestResult<u64> {
    Ok(client.inventory().ok_or("gold-inventory-absent")?.entries.iter()
        .map(|entry| {
            coins.iter().find(|(key, _)| *key == entry.item_definition_ref)
                .map_or(0, |(_, value)| u64::from(entry.count.get()) * value)
        })
        .sum())
}

pub(super) async fn earn_gold(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    needed: u64,
) -> TestResult {
    let coins = coin_references()?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while carried_gold(client, &coins)? < needed {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "gold-combat-budget:observed={}:needed={needed}",
                carried_gold(client, &coins)?
            )
            .into());
        }
        travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(client).await?;
    }
    Ok(())
}

async fn walk_ranged_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction);
        let mut step = client.step(direction).await?;
        if step.disposition == StepDisposition::Blocked {
            let occupied = expected.ok_or("C02 blocked step has no compiled destination")?;
            let entities = client.world_entities().ok_or("C02 occupancy view absent")?;
            let occupant = entities.iter()
                .find(|entity| tile(entity.position).ok() == Some(occupied))
                .ok_or_else(|| format!(
                    "BLOCKER-SERVER C02 blocked tile has no occupant: from={from:?} tile={occupied:?} reply={step:?}"
                ))?;
            if occupant.kind != EntityKind::Creature {
                return Err(format!("C02 blocked by non-creature occupant: {:?}", occupant.entity).into());
            }
            let occupant = occupant.entity;
            evidence(&format!("C02_STEP_BLOCKED creature={occupant:?} tile={occupied:?} reply={step:?}"));
            let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
            verdict::Harness::global().bounded_wait(deadline, "C02 blocked creature");
            loop {
                client.service_liveness(Duration::from_millis(25)).await?;
                let view = client.world_entities().ok_or("C02 occupancy view absent")?;
                if view.get(&occupant).is_none_or(|entity| tile(entity.position).ok() != Some(occupied)) {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("C02 creature {occupant:?} still occupies {occupied:?}").into());
                }
                client.take_events();
            }
            evidence(&format!("C02_STEP_CLEAR creature={occupant:?} tile={occupied:?}"));
            step = client.step(direction).await?;
        }
        let arrived = tile(client.world_spatial().actor_position)?;
        if step.disposition != StepDisposition::Moved
            || step.world_spatial_delta.is_none()
            || expected != Some(arrived)
        {
            return Err(format!(
                "C02 step refused or misplaced: direction={direction:?} from={from:?} expected={expected:?} actual={arrived:?} reply={step:?}"
            ).into());
        }
        verdict::Harness::global().progress("walk");
        client
            .service_liveness(Duration::from_millis(u64::from(step.step_wait_ms)))
            .await?;
        client.take_events();
    }
    Ok(())
}

pub(super) async fn approach_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    range: u16,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let rat_points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|entry| entry.key == "oteryn:creature.rat")
        })
        .collect();
    let path = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|cell| !cell.protection_zone())
            && rat_points.iter().any(|(floor, point)| {
                at.floor == *floor && at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) <= range
            })
    })
    .ok_or("C02 no reachable native firing position for an authored Rat spawn")?;
    walk_ranged_route(client, world, &path).await
}

async fn buy(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    id: u32,
    quantity: u32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop;
    let key = reference(id)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let item_key = format!("oteryn:item.tibia.i{id}");
    let sellers: BTreeSet<String> = catalog["records"]
        .as_array()
        .ok_or("trade-records-absent")?
        .iter()
        .filter_map(|r| {
            r["declaration"]["offers"]
                .as_array()?
                .iter()
                .any(|o| o["direction"] == "SellToPlayer" && o["item"]["key"] == item_key)
                .then(|| {
                    r["declaration"]["identity"]["key"]
                        .as_str()?
                        .strip_prefix("oteryn:service.trade.")
                        .map(|n| format!("oteryn:npc.{n}"))
                })?
        })
        .collect();
    let here = tile(client.world_spatial().actor_position)?;
    let (seller, path) = npcs
        .npcs
        .iter()
        .filter(|n| sellers.contains(&n.key))
        .filter_map(|n| {
            let approach = npc::TALK_RANGE_TILES - if roaming_seller_name(&n.key).is_some() { 2 } else { 0 };
            route(world, here, |at| {
                n.placements.iter().any(|p| {
                    p.floor == at.floor
                        && u32::from(at.x.abs_diff(p.x).max(at.y.abs_diff(p.y))) <= approach
                })
            })
            .map(|p| (n, p))
        })
        .min_by_key(|(_, p)| p.len())
        .ok_or_else(|| format!("reachable-seller-absent:item={id}"))?;
    walk_ranged_route(client, world, &path).await?;
    let player = client.world_spatial().actor_position;
    let actor = client
        .world_entities()
        .and_then(|view| {
            let mut candidates = view.iter().filter(|entity| {
                entity.kind == EntityKind::Npc
                    && entity.position.floor == player.floor
                    && distance(entity.position, player) <= npc::TALK_RANGE_TILES
                    && seller.placements.iter().any(|p| {
                        let home = ActorPosition {
                            x: i32::from(p.x),
                            y: i32::from(p.y),
                            floor: -i16::from(p.floor),
                        };
                        seller_position_matches(&seller.key, home, entity.position)
                            && roaming_seller_name(&seller.key).is_none_or(|expected| {
                                entity.display_name.is_some_and(|name| {
                                    name.as_str().is_ok_and(|name| name == expected)
                                })
                            })
                    })
            });
            let candidate = candidates.next()?;
            candidates.next().is_none().then_some(candidate.entity)
        })
        .ok_or_else(|| {
            format!(
                "need=unique-visible-seller-within-talk-range;seller={}",
                seller.key
            )
        })?;
    for text in ["hi", "trade"] {
        let reply = npc_shop::talk(
            client.session_mut(),
            &npc::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!("trade-talk:{text}:reply={reply:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let details = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
    let window = npc::decode_npc_trade_window_for(&delta.payload, details)
        .map_err(|e| format!("trade-window:{e:?}"))?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| o.side == npc::NpcTradeSide::Buy && o.item_definition_ref == key.get())
        .ok_or("live-offer-absent")?;
    let before = count(client, key)?;
    let coins = coin_references()?;
    let gold_before = carried_gold(client, &coins)?;
    let (seller_actor, catalogue_revision) = window.open.ok_or("trade-window-closed")?;
    if seller_actor != actor {
        return Err("C02 trade window belongs to a different seller".into());
    }
    let reply = npc_shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("trade-buy:item={id}:reply={reply:?}").into());
    }
    let sequence = wait_count(
        client,
        key,
        before + u64::from(offer.count) * u64::from(quantity),
    )
    .await?;
    let paid = u64::from(offer.unit_price) * u64::from(quantity);
    let gold_after = carried_gold(client, &coins)?;
    if gold_before.checked_sub(paid) != Some(gold_after) {
        return Err(format!(
            "C02 trade debit mismatch: item={id} gold={gold_before}->{gold_after} expected_debit={paid}"
        ).into());
    }
    Ok(sequence)
}

fn distance(a: ActorPosition, b: ActorPosition) -> u32 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}

fn roaming_seller_name(key: &str) -> Option<&'static str> {
    match key {
        "oteryn:npc.galuna" => Some("Galuna"),
        "oteryn:npc.xodet" => Some("Xodet"),
        _ => None,
    }
}

fn seller_position_matches(key: &str, home: ActorPosition, position: ActorPosition) -> bool {
    position.floor == home.floor
        && (position == home || (roaming_seller_name(key).is_some() && distance(home, position) <= 2))
}

pub(super) fn frame_follows_result(frame: u64, result: u64) -> bool {
    frame > result
}

async fn wait_moved_inventory(
    client: &mut DevClientSession,
    reply: oteryn_dev_client::ItemMoveOutcomeResult,
    matches_destination: impl Fn(&items::CharacterInventory) -> bool,
) -> TestResult {
    if reply.status != CommandStatus::Accepted || reply.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("item-move-refused:{reply:?}").into());
    }
    tokio::time::timeout(Duration::from_secs(6), async {
        loop {
            for event in client.take_events() {
                if let SessionEvent::Inventory(delta) = event
                    && frame_follows_result(delta.server_sequence, reply.result_server_sequence)
                    && matches_destination(&delta.value)
                {
                    return TestResult::Ok(());
                }
            }
            client.service_liveness(Duration::from_millis(25)).await?;
        }
    })
    .await
    .map_err(|_| {
        format!(
            "inventory-destination-delta-missing-after={}",
            reply.result_server_sequence
        )
    })?
}

pub(super) async fn clear_bow_offhand(client: &mut DevClientSession) -> TestResult {
    let inventory = client.inventory().ok_or("bow-offhand-inventory-absent")?;
    let Some(offhand) = inventory
        .equipment
        .iter()
        .find(|entry| entry.slot == EquipmentSlot::LeftHand)
    else {
        return Ok(());
    };
    let handle = offhand.item.handle;
    client.take_events();
    let moved = client
        .move_item(&items::ItemMoveIntent {
            source: handle,
            destination: items::ItemMoveDestination::MainBackpack,
        })
        .await?;
    wait_moved_inventory(client, moved, |view| {
        !view
            .equipment
            .iter()
            .any(|entry| entry.slot == EquipmentSlot::LeftHand)
            && view.entries.iter().any(|entry| entry.handle == handle)
    })
    .await
}

pub(super) async fn equip(
    client: &mut DevClientSession,
    key: NonZeroU32,
    slot: EquipmentSlot,
) -> TestResult {
    let source = carried(client, key)?;
    client.take_events();
    let reply = client
        .move_item(&items::ItemMoveIntent {
            source,
            destination: items::ItemMoveDestination::Equipment(slot),
        })
        .await?;
    wait_moved_inventory(client, reply, |view| {
        view.equipment
            .iter()
            .any(|entry| entry.slot == slot && entry.item.item_definition_ref == key)
    })
    .await
}

fn own(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let view = client.world_entities().ok_or("entity-view-absent")?;
    view.iter()
        .find(|e| &e.entity.identity == view.own_identity())
        .map(|e| e.entity)
        .ok_or_else(|| "own-entity-absent".into())
}
fn target(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let here = client.world_spatial().actor_position;
    client
        .world_entities()
        .and_then(|v| {
            v.iter()
                .filter(|e| {
                    e.kind == EntityKind::Creature
                        && e.position.floor == here.floor
                        && distance(e.position, here) <= 1
                        && e.display_name.is_some_and(|name| {
                            name.as_str()
                                .is_ok_and(|name| name.eq_ignore_ascii_case("rat"))
                        })
                })
                .min_by_key(|e| {
                    e.position
                        .x
                        .abs_diff(here.x)
                        .max(e.position.y.abs_diff(here.y))
                })
        })
        .map(|e| e.entity)
        .ok_or_else(|| "live-target-absent".into())
}
fn target_health(client: &DevClientSession, target: oteryn_dev_client::EntityRef) -> Option<u8> {
    match client.world_entities()?.get(&target)?.detail {
        oteryn_dev_client::EntityDetail::Actor { health_percent, .. } => Some(health_percent),
        _ => None,
    }
}

async fn attack_damage(
    client: &mut DevClientSession,
    color: Option<PresentationColor>,
) -> TestResult<(u64, u32, u32)> {
    let attacker = own(client)?;
    let victim = target(client)?;
    let before_health = target_health(client, victim).ok_or("target-health-absent")?;
    let before_mana = client
        .actor_vitals()
        .ok_or("pre-attack-vitals-absent")?
        .mana;
    client.take_events();
    let reply = client.session_mut().attack_target(Some(&victim)).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("attack-reply={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    let mut damage_sequence = None;
    loop {
        client.service_liveness(Duration::from_millis(40)).await?;
        for event in client.take_events() {
            if let SessionEvent::Presentation(delta) = event
                && delta.value.iter().any(|event| matches!(event.kind,
                    PresentationEventKind::ValueText { attacker: Some(a), target: t, primary, secondary, .. }
                        if a == attacker && t == victim && primary.value > 0
                            && color.is_none_or(|c| primary.color == c || secondary.is_some_and(|v| v.color == c))))
            {
                damage_sequence = Some(delta.server_sequence);
            }
        }
        if let Some(sequence) = damage_sequence
            && target_health(client, victim).is_none_or(|health| health < before_health)
        {
            let after = client
                .actor_vitals()
                .ok_or("post-damage-vitals-absent")?
                .mana;
            return Ok((sequence, before_mana, after));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("damage-or-hp-delta-timeout:attacker={attacker:?}:target={victim:?}:color={color:?}:reply={reply:?}").into());
        }
    }
}
fn bow_debit(previous: &mut u64, current: u64) -> bool {
    if previous.checked_sub(current) != Some(1) {
        return false;
    }
    *previous = current;
    true
}

#[test]
fn bow_miss_then_hit_spends_one_arrow_per_attempt() {
    let mut remaining = 10;
    assert!(bow_debit(&mut remaining, 9));
    assert!(bow_debit(&mut remaining, 8));
    assert!(!bow_debit(&mut remaining, 6));
    assert!(!bow_debit(&mut remaining, 9));
    assert_eq!(remaining, 8);
}

async fn bow_shot(client: &mut DevClientSession, arrows: NonZeroU32) -> TestResult<(u64, u64)> {
    let attacker = own(client)?;
    let victim = target(client)?;
    let before_health = target_health(client, victim).ok_or("bow-target-health-absent")?;
    let before = count(client, arrows)?;
    client.take_events();
    let reply = client.session_mut().attack_target(Some(&victim)).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("bow-attack-reply={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    let mut inventory_sequence = 0;
    let mut remaining = before;
    let mut damage_sequence = 0;
    loop {
        client.service_liveness(Duration::from_millis(25)).await?;
        for event in client.take_events() {
            match event {
                SessionEvent::Inventory(delta) => {
                    let observed = inventory_count(&delta.value, arrows);
                    if observed != remaining {
                        if !bow_debit(&mut remaining, observed) {
                            return Err(format!("bow-invalid-arrow-debit:remaining={remaining}:observed={observed}").into());
                        }
                        inventory_sequence = delta.server_sequence;
                    }
                }
                SessionEvent::Presentation(delta)
                    if delta.value.iter().any(|event| matches!(event.kind,
                        PresentationEventKind::ValueText { attacker: Some(a), target: t, primary, .. }
                            if a == attacker && t == victim && primary.value > 0)) =>
                {
                    damage_sequence = delta.server_sequence;
                }
                _ => {}
            }
        }
        if inventory_sequence != 0
            && damage_sequence != 0
            && target_health(client, victim).is_none_or(|health| health < before_health)
        {
            return Ok((inventory_sequence, damage_sequence));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("bow-shot-timeout:reply={reply:?}:before={before}:after={}:inventory-sequence={inventory_sequence}:damage-sequence={damage_sequence}", count(client,arrows)?).into());
        }
    }
}
async fn consume_one_arrow(client: &mut DevClientSession, arrows: NonZeroU32) -> TestResult {
    let victim = target(client)?;
    let before = count(client, arrows)?;
    client.take_events();
    let reply = client.session_mut().attack_target(Some(&victim)).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("arrow-drain-attack-reply={reply:?}").into());
    }
    wait_count(client, arrows, before - 1).await?;
    stop(client).await
}
pub(super) async fn stop(client: &mut DevClientSession) -> TestResult {
    let target = client.session_mut().combat_state().ok_or("C02 combat state absent")?.target;
    let Some(target) = target else {
        return Ok(());
    };
    let reply = client.session_mut().attack_target(None).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("stop-reply={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    loop {
        for event in client.take_events() {
            if let SessionEvent::ActorCombatState(delta) = event
                && delta.server_sequence > reply.result_server_sequence
                && delta.value.target.is_none()
            {
                evidence(&format!("C02_ATTACK_CLEARED target={target:?} sequence={}", delta.server_sequence));
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C02 attack target {target:?} did not clear").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn prepare_ranged_character(
    clients: &SeamClients<'_>,
    profile: &'static str,
    vocation: &str,
    platinum: u32,
    tag: u8,
) -> TestResult<DevClientSession> {
    if !requires_platform_grant(&clients.accounts[0]) {
        return Err(PLATFORM_ADMISSION_GAP.into());
    }
    let mut client = connect_character(clients, tag, 0).await?;
    let old_session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(old_session.as_bytes()),
        "--profile",
        profile.into(),
    )
    .await?;
    character_prep::grant_item(&client, 3035, platinum).await?;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, tag, 0).await?;
    let stats = general_stats(&mut fresh).await?;
    let vitals = fresh.actor_vitals().ok_or("seeded ranged vitals absent")?;
    let experience = crate::qualification_spell_character::source_experience(8)?;
    if fresh.join_snapshot().game_session_id == old_session
        || stats.last().copied()
            != oteryn_protocol_oteryn::cyclopedia_character::vocation_id(vocation)
        || stats.get(1..9) != Some(experience.to_le_bytes().as_slice())
        || stats.get(9..13) != Some(8_u32.to_le_bytes().as_slice())
        || (vitals.max_health, vitals.max_mana) != (185, 90)
        || count(&fresh, reference(3035)?)? != u64::from(platinum)
    {
        return Err(format!(
            "seeded ranged character mismatch: vocation={vocation} vitals={vitals:?}"
        )
        .into());
    }
    evidence(&format!(
        "C02_PREPARED vocation={vocation} level=8 xp={experience} max_health=185 max_mana=90 platinum={platinum}"
    ));
    Ok(fresh)
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    character_prep::declare_seeded(
        "C02.full",
        "separate level-8 Sorcerer and Druid with NPC purchase funds",
        "ordinary leveling and funding exceed the ranged row budget",
    );
    let sorcerer = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_C02_ACCOUNT",
        Some("Ranged Sorcerer"),
    )
    .await?;
    let sorcerer_clients = sorcerer.clients(clients);
    let mut client =
        prepare_ranged_character(&sorcerer_clients, "c02-sorcerer", "sorcerer", 10, 0xc2).await?;
    let mut purchase_sequences = Vec::new();
    for (id, n) in [(BOW, 1), (ARROW, 10), (WAND, 1)] {
        purchase_sequences.push(
            buy(&mut client, world, &metadata.npcs, id, n)
                .await
                .map_err(|error| format!("purchase-item-{id}: {error}"))?,
        );
    }
    row.pass(1, purchase_sequences);
    let bow = reference(BOW)?;
    let arrow = reference(ARROW)?;
    let wand = reference(WAND)?;
    let rod = reference(ROD)?;
    clear_bow_offhand(&mut client).await?;
    equip(&mut client, bow, EquipmentSlot::RightHand).await?;
    equip(&mut client, arrow, EquipmentSlot::Ammo).await?;
    approach_rat(&mut client, world, &metadata.spawns, 1).await?;
    let bow_xp_before = experience(&mut client).await?;
    let damage = bow_shot(&mut client, arrow).await?;
    stop(&mut client).await?;
    while count(&client, arrow)? > 0 {
        approach_rat(&mut client, world, &metadata.spawns, 1).await?;
        consume_one_arrow(&mut client, arrow).await?;
    }
    approach_rat(&mut client, world, &metadata.spawns, 1).await?;
    let bow_xp = experience(&mut client).await?;
    if bow_xp <= bow_xp_before {
        return Err(
            format!("bow-kill-unproven:arrows-exhausted:xp={bow_xp_before}->{bow_xp}").into(),
        );
    }
    let state = client.inventory().cloned();
    let mana = client.actor_vitals().ok_or("no-ammo-vitals-absent")?.mana;
    client.take_events();
    let refusal_target = target(&client)?;
    let refusal_health =
        target_health(&client, refusal_target).ok_or("no-ammo-target-health-absent")?;
    let refusal_attacker = own(&client)?;
    let refusal = client
        .session_mut()
        .attack_target(Some(&refusal_target))
        .await?;
    if refusal.disposition != AttackIntentDisposition::Ok {
        return Err(format!("no-ammo-intent-reply={refusal:?}").into());
    }
    client.service_liveness(Duration::from_millis(2200)).await?;
    let attributed=client.take_events().iter().any(|e| matches!(e,SessionEvent::Presentation(d) if d.value.iter().any(|p| matches!(p.kind,PresentationEventKind::ValueText{attacker:Some(a),..} if a==refusal_attacker))));
    if attributed
        || target_health(&client, refusal_target) != Some(refusal_health)
        || client.actor_vitals().is_none_or(|v| v.mana != mana)
        || client.inventory() != state.as_ref()
    {
        return Err(
            format!("no-ammo-refusal-mutated:reply={refusal:?}:damage={attributed}").into(),
        );
    }
    stop(&mut client).await?;
    row.pass(2, (damage, refusal.disposition, bow_xp));
    equip(&mut client, wand, EquipmentSlot::RightHand).await?;
    approach_rat(&mut client, world, &metadata.spawns, 1).await?;
    let wand_count = count(&client, wand)?;
    let wand_xp_before = experience(&mut client).await?;
    let wand_hit = attack_damage(&mut client, Some(PresentationColor::Purple)).await?;
    if wand_hit.1.checked_sub(WAND_MANA) != Some(wand_hit.2) || count(&client, wand)? != wand_count
    {
        return Err(format!("wand-cost-or-custody:observed={wand_hit:?}").into());
    }
    let wand_xp = finish_kill(&mut client, wand_xp_before, "wand").await?;
    row.pass(3, (wand_hit, wand_xp));
    retreat_from_combat(&mut client, world).await?;
    let old = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    let expected_mana = client.actor_vitals().ok_or("logout-vitals-absent")?.mana;
    drop(client);
    let mut fresh = connect_character(&sorcerer_clients, 0xc3, 0).await?;
    let admitted_xp = experience(&mut fresh).await?;
    if fresh.join_snapshot().game_session_id == old
        || !durable_rewards_match(wand_xp, admitted_xp)
        || count(&fresh, arrow)? != 0
        || [bow, wand]
            .into_iter()
            .any(|k| count(&fresh, k).ok() != Some(1))
        || fresh.actor_vitals().is_none_or(|v| v.mana != expected_mana)
    {
        return Err(
            format!("fresh-admission-durable-state-mismatch:xp={wand_xp}->{admitted_xp}").into(),
        );
    }
    logout(&mut fresh).await?;
    drop(fresh);

    // The rod leg uses a distinct character and proves its own purchase and XP.
    let druid = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_C02_ACCOUNT",
        Some("Ranged Druid"),
    )
    .await?;
    let druid_clients = druid.clients(clients);
    let mut client =
        prepare_ranged_character(&druid_clients, "c02-druid", "druid", 5, 0xc4).await?;
    require_eligible_druid(&mut client).await?;
    buy(&mut client, world, &metadata.npcs, ROD, 1).await?;
    equip(&mut client, rod, EquipmentSlot::RightHand)
        .await
        .map_err(|error| {
            format!("equip-Snakebite-Rod-{ROD}: {error}; need=rod-eligible-druid-level-6")
        })?;
    approach_rat(&mut client, world, &metadata.spawns, 1).await?;
    let rod_count = count(&client, rod)?;
    let rod_xp_before = experience(&mut client).await?;
    let rod_hit = attack_damage(&mut client, Some(PresentationColor::LightGreen)).await?;
    if rod_hit.1.checked_sub(ROD_MANA) != Some(rod_hit.2) || count(&client, rod)? != rod_count {
        return Err(format!("rod-cost-or-custody:observed={rod_hit:?}").into());
    }
    let rod_xp = finish_kill(&mut client, rod_xp_before, "rod").await?;
    row.pass(4, (rod_hit, rod_xp));
    retreat_from_combat(&mut client, world).await?;
    let old = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    let expected_mana = client
        .actor_vitals()
        .ok_or("rod-logout-vitals-absent")?
        .mana;
    drop(client);
    let mut fresh = connect_character(&druid_clients, 0xc5, 0).await?;
    let admitted_xp = experience(&mut fresh).await?;
    if fresh.join_snapshot().game_session_id == old
        || !durable_rewards_match(rod_xp, admitted_xp)
        || count(&fresh, rod)? != rod_count
        || fresh.actor_vitals().is_none_or(|v| v.mana != expected_mana)
    {
        return Err(format!(
            "druid-fresh-admission-durable-state-mismatch:xp={rod_xp}->{admitted_xp}"
        )
        .into());
    }
    row.pass(5, (wand_xp, rod_xp, admitted_xp));
    logout(&mut fresh).await
}

async fn general_stats(client: &mut DevClientSession) -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let tab = wire::CharacterTab::GeneralStats as u8;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[tab, 0, 0, 50],
        )
        .await?;
    if reply.status != CommandStatus::Accepted || reply.payload.first() != Some(&tab) {
        return Err(format!("GeneralStats refused: {reply:?}").into());
    }
    Ok(reply.payload)
}

async fn require_eligible_druid(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_CYCLOPEDIA_CHARACTER_VOCATION_V1)
    {
        return Err("need=served-vocation-for-separate-level-6-druid".into());
    }
    let stats = general_stats(client).await?;
    let vocation = stats.last().copied();
    let level = u32::from_le_bytes(
        stats
            .get(9..13)
            .ok_or("GeneralStats level missing")?
            .try_into()?,
    );
    if !["druid", "elder_druid"]
        .into_iter()
        .any(|name| wire::vocation_id(name) == vocation)
        || level < 6
    {
        return Err(format!(
            "need=separate-level-6-druid;observed-vocation={vocation:?};level={level}"
        )
        .into());
    }
    Ok(())
}

async fn experience(client: &mut DevClientSession) -> TestResult<u64> {
    let payload = general_stats(client).await?;
    Ok(u64::from_le_bytes(
        payload
            .get(1..9)
            .ok_or("GeneralStats XP missing")?
            .try_into()?,
    ))
}

async fn finish_kill(client: &mut DevClientSession, before: u64, weapon: &str) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let xp = experience(client).await?;
        if xp > before {
            stop(client).await?;
            return Ok(xp);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("{weapon}-kill-unproven:xp={before}->{xp}:deadline=20s").into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
        client.take_events();
    }
}

pub(super) async fn retreat_from_combat(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|cell| cell.protection_zone())
    }).ok_or("C02 no reachable protection zone for logout")?;
    walk_ranged_route(client, world, &path).await
}

pub(super) async fn logout(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::logout::{LogoutResult, MAX_LOGOUT_RETRY_AFTER_MS};
    let duration = Duration::from_millis(u64::from(MAX_LOGOUT_RETRY_AFTER_MS));
    let deadline = tokio::time::Instant::now() + duration;
    while client.session_mut().combat_state().ok_or("C02 combat state absent before logout")?.in_fight {
        if tokio::time::Instant::now() >= deadline {
            return Err("C02 combat lock did not clear before the logout deadline".into());
        }
        verdict::Harness::global().bounded_wait(deadline, "combat-logout-lock");
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(LogoutResult::Accepted) {
        return Err(format!("logout-reply={reply:?}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c02_rejects_lost_kill_experience_after_readmission() {
        assert!(durable_rewards_match(105, 105));
        assert!(!durable_rewards_match(105, 100));
    }

    #[test]
    fn ranged_ids_are_distinct_and_resolve() -> TestResult {
        let ids = [BOW, ARROW, WAND, ROD].map(reference);
        let ids: Vec<_> = ids.into_iter().collect::<Result<_, _>>()?;
        assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), 4);
        Ok(())
    }
}

#[test]
fn c02_wand_and_rod_use_their_own_mana_costs() {
    assert_eq!((WAND_MANA, ROD_MANA), (2, 1));
}

#[test]
fn c02_roaming_sellers_stay_on_their_authored_floor() {
    let home = ActorPosition {
        x: 33196,
        y: 32817,
        floor: 7,
    };
    let moved = ActorPosition {
        x: home.x + 2,
        ..home
    };
    assert!(seller_position_matches("oteryn:npc.xodet", home, moved));
    assert!(seller_position_matches("oteryn:npc.galuna", home, moved));
    assert!(!seller_position_matches("oteryn:npc.other", home, moved));
    assert!(!seller_position_matches(
        "oteryn:npc.xodet",
        home,
        ActorPosition { floor: 8, ..moved }
    ));
}

#[test]
fn c02_destination_proof_requires_a_newer_frame() {
    assert!(!frame_follows_result(9, 10));
    assert!(!frame_follows_result(10, 10));
    assert!(frame_follows_result(11, 10));
}
