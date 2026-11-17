//! C02 full-row journey, derived from the matrix, re-audit, and `attack/ranged.rs` owner.
//!
//! 1. Admit the provisioned character, earn spendable gold through ordinary rat combat/loot,
//!    walk to compiled-world sellers, say `hi` and `trade`, and buy a bow, ten arrows, a Wand of
//!    Vortex, and a Snakebite Rod; require every accepted trade reply and exact inventory delta.
//! 2. Equip the bow and arrows with ITEM_MOVE, send ATTACK against a live rat, and require an Ok
//!    attack reply, an exact one-arrow inventory decrement, and a damage presentation attributed
//!    from this character to that rat. Continue firing until all arrows are consumed, then send
//!    ATTACK again and require the Ok intent reply but no damage presentation, mana debit, or
//!    inventory change during a complete attack interval (the server's no-ammunition refusal).
//! 3. Equip the Wand of Vortex, send ATTACK against a live rat, and require the Ok reply, an exact
//!    two-mana debit, and attributed energy-coloured target damage without inventory consumption.
//! 4. Equip the Snakebite Rod, send ATTACK against a live rat, and require the Ok reply, an exact
//!    two-mana debit, and attributed earth-coloured target damage without inventory consumption.
//! 5. Stop attacking, logout, freshly admit the character, and require a new game session plus
//!    the durable zero-arrow count, owned bow/wand/rod, and both wand/rod mana payments.

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
const SHOT_MANA: u32 = 2;

struct Row {
    started: std::time::Instant,
    step: usize,
}
impl Row {
    fn pass(&mut self, step: usize, observed: impl std::fmt::Debug) {
        self.step = step;
        evidence(&format!("C02_CHECK step={step} PASS observed={observed:?}"));
    }
    fn finish(self, result: TestResult) {
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
    let result = verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(&mut row, clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err(format!("timeout-after-step-{}", row.step).into()),
    };
    row.finish(result);
}

fn reference(id: u32) -> TestResult<NonZeroU32> {
    provisioning::item_reference(&provisioning::item_index()?, id)
}
fn count(client: &DevClientSession, key: NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("inventory-frame-absent")?
        .entries
        .iter()
        .chain(
            client
                .inventory()
                .into_iter()
                .flat_map(|v| v.equipment.iter().map(|e| &e.item)),
        )
        .filter(|e| e.item_definition_ref == key)
        .map(|e| u64::from(e.count.get()))
        .sum())
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

async fn earn_gold(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    needed: u64,
) -> TestResult {
    let gold = [
        (reference(3031)?, 1),
        (reference(3035)?, 100),
        (reference(3043)?, 10_000),
    ];
    let value = |client: &DevClientSession| -> TestResult<u64> {
        Ok(client
            .inventory()
            .ok_or("gold-inventory-absent")?
            .entries
            .iter()
            .map(|e| {
                gold.iter()
                    .find(|(k, _)| *k == e.item_definition_ref)
                    .map_or(0, |(_, v)| u64::from(e.count.get()) * v)
            })
            .sum())
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while value(client)? < needed {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "gold-combat-budget:observed={}:needed={needed}",
                value(client)?
            )
            .into());
        }
        travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(client).await?;
    }
    Ok(())
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
            route(world, here, |at| {
                n.placements
                    .iter()
                    .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
            })
            .map(|p| (n, p))
        })
        .min_by_key(|(_, p)| p.len())
        .ok_or_else(|| format!("reachable-seller-absent:item={id}"))?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|v| {
            v.iter().find(|e| {
                e.kind == EntityKind::Npc
                    && seller.placements.iter().any(|p| {
                        e.position.x == i32::from(p.x)
                            && e.position.y == i32::from(p.y)
                            && e.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|e| e.entity)
        .ok_or("seller-entity-absent")?;
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
    let window =
        npc::decode_npc_trade_window(&delta.payload).map_err(|e| format!("trade-window:{e:?}"))?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| o.side == npc::NpcTradeSide::Buy && o.item_definition_ref == key.get())
        .ok_or("live-offer-absent")?;
    let before = count(client, key)?;
    let reply = npc_shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade-window-closed")?.1,
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
    wait_count(
        client,
        key,
        before + u64::from(offer.count) * u64::from(quantity),
    )
    .await
}

async fn equip(client: &mut DevClientSession, key: NonZeroU32, slot: EquipmentSlot) -> TestResult {
    let reply = client
        .move_item(&items::ItemMoveIntent {
            source: carried(client, key)?,
            destination: items::ItemMoveDestination::Equipment(slot),
        })
        .await?;
    if reply.outcome != items::ItemMoveOutcome::Moved
        || client.inventory().is_none_or(|v| {
            !v.equipment
                .iter()
                .any(|e| e.slot == slot && e.item.item_definition_ref == key)
        })
    {
        return Err(format!("equip:item={key}:slot={slot:?}:reply={reply:?}").into());
    }
    Ok(())
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
                .filter(|e| e.kind == EntityKind::Creature && e.position.floor == here.floor)
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
async fn attack_damage(
    client: &mut DevClientSession,
    color: Option<PresentationColor>,
) -> TestResult<(u64, u32, u32)> {
    let attacker = own(client)?;
    let victim = target(client)?;
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
    loop {
        client.service_liveness(Duration::from_millis(40)).await?;
        for event in client.take_events() {
            if let SessionEvent::Presentation(delta) = event && delta.value.iter().any(|e| matches!(e.kind, PresentationEventKind::ValueText { attacker: Some(a), target: t, primary, secondary, .. } if a == attacker && t == victim && primary.value > 0 && color.is_none_or(|c| primary.color == c || secondary.is_some_and(|v| v.color == c)))) { let after = client.actor_vitals().ok_or("post-damage-vitals-absent")?.mana; return Ok((delta.server_sequence,before_mana,after)); }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("damage-timeout:attacker={attacker:?}:target={victim:?}:color={color:?}:reply={reply:?}").into());
        }
    }
}
async fn bow_shot(client: &mut DevClientSession, arrows: NonZeroU32) -> TestResult<(u64, u64)> {
    let attacker = own(client)?;
    let victim = target(client)?;
    let before = count(client, arrows)?;
    client.take_events();
    let reply = client.session_mut().attack_target(Some(&victim)).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("bow-attack-reply={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    let mut inventory_sequence = 0;
    let mut damage_sequence = 0;
    loop {
        client.service_liveness(Duration::from_millis(25)).await?;
        for event in client.take_events() {
            match event {
                SessionEvent::Inventory(delta) if count(client, arrows)? + 1 == before => {
                    inventory_sequence = delta.server_sequence;
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
        if inventory_sequence != 0 && damage_sequence != 0 {
            return Ok((inventory_sequence, damage_sequence));
        }
        if count(client, arrows)? + 1 < before {
            return Err(format!(
                "bow-shot-consumed-multiple:before={before}:after={}",
                count(client, arrows)?
            )
            .into());
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
async fn stop(client: &mut DevClientSession) -> TestResult {
    let r = client.session_mut().attack_target(None).await?;
    if r.disposition != AttackIntentDisposition::Ok {
        return Err(format!("stop-reply={r:?}").into());
    }
    Ok(())
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
    let mut client = connect_world(clients, 0xc2).await?;
    earn_gold(&mut client, world, &metadata.spawns, 1430).await?;
    let mut purchase_sequences = Vec::new();
    for (id, n) in [(BOW, 1), (ARROW, 10), (WAND, 1), (ROD, 1)] {
        purchase_sequences.push(buy(&mut client, world, &metadata.npcs, id, n).await?);
    }
    row.pass(1, purchase_sequences);
    let bow = reference(BOW)?;
    let arrow = reference(ARROW)?;
    let wand = reference(WAND)?;
    let rod = reference(ROD)?;
    equip(&mut client, bow, EquipmentSlot::RightHand).await?;
    equip(&mut client, arrow, EquipmentSlot::Ammo).await?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let damage = bow_shot(&mut client, arrow).await?;
    stop(&mut client).await?;
    while count(&client, arrow)? > 0 {
        travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        consume_one_arrow(&mut client, arrow).await?;
    }
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let state = client.inventory().cloned();
    let mana = client.actor_vitals().ok_or("no-ammo-vitals-absent")?.mana;
    client.take_events();
    let refusal_target = target(&client)?;
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
        || client.actor_vitals().is_none_or(|v| v.mana != mana)
        || client.inventory() != state.as_ref()
    {
        return Err(
            format!("no-ammo-refusal-mutated:reply={refusal:?}:damage={attributed}").into(),
        );
    }
    stop(&mut client).await?;
    row.pass(2, (damage, refusal.disposition));
    equip(&mut client, wand, EquipmentSlot::RightHand).await?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let wand_count = count(&client, wand)?;
    let wand_hit = attack_damage(&mut client, Some(PresentationColor::Purple)).await?;
    if wand_hit.1.checked_sub(SHOT_MANA) != Some(wand_hit.2) || count(&client, wand)? != wand_count
    {
        return Err(format!("wand-cost-or-custody:observed={wand_hit:?}").into());
    }
    stop(&mut client).await?;
    row.pass(3, wand_hit);
    equip(&mut client, rod, EquipmentSlot::RightHand).await?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let rod_count = count(&client, rod)?;
    let rod_hit = attack_damage(&mut client, Some(PresentationColor::LightGreen)).await?;
    if rod_hit.1.checked_sub(SHOT_MANA) != Some(rod_hit.2) || count(&client, rod)? != rod_count {
        return Err(format!("rod-cost-or-custody:observed={rod_hit:?}").into());
    }
    stop(&mut client).await?;
    row.pass(4, rod_hit);
    let expected_mana = rod_hit.2;
    let old = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xc3).await?;
    if fresh.join_snapshot().game_session_id == old
        || count(&fresh, arrow)? != 0
        || [bow, wand, rod]
            .into_iter()
            .any(|k| count(&fresh, k).ok() != Some(1))
        || fresh.actor_vitals().is_none_or(|v| v.mana != expected_mana)
    {
        return Err("fresh-admission-durable-state-mismatch".into());
    }
    row.pass(5, (fresh.join_snapshot().game_session_id, expected_mana));
    logout(&mut fresh).await
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let r = client.session_mut().logout().await?;
        if r.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("logout-reply={:?}", r.result).into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranged_ids_are_distinct_and_resolve() -> TestResult {
        let ids = [BOW, ARROW, WAND, ROD].map(reference);
        let ids: Vec<_> = ids.into_iter().collect::<Result<_, _>>()?;
        assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), 4);
        Ok(())
    }
}
