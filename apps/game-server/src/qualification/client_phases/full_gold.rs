//! Shared real-play gold for full rows: pick the reachable creature that pays best for this
//! character, hunt it, loot every coin stack, buy from a catalogue seller and equip, with a
//! target the caller computes from what its row actually buys.
//!
//! The creature is chosen from the bundle's spawns, the loot tables and the creature profiles:
//! expected coin gold per kill over the seconds a kill takes, among creatures whose health the
//! character's level can safely kill. Every kill and loot move counts as progress of the running
//! row (the 90 s no-progress watchdog), and every step logs observed against needed gold.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::{
    item_view::{self as items, EquipmentSlot},
    npc_service as npc,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
};

const COINS: [(u32, u64); 3] = [(3031, 1), (3035, 100), (3043, 10_000)];
/// A creature is safe to hunt when its health is at most this many times the character's level
/// (never below [`MIN_HEALTH_CAP`]).
const HEALTH_PER_LEVEL: u64 = 10;
const MIN_HEALTH_CAP: u64 = 20;
/// Overhead of one kill (walk to the spawn, corpse, loot) before the damage time.
const KILL_OVERHEAD_SECONDS: u64 = 12;
/// Health the character removes per second is taken as `2 + level`.
const SETUP_SECONDS: u64 = 60;
/// Wall time a row grants its outer deadline for earning, whatever the need.
const ALLOWANCE_CEILING_SECONDS: u64 = 900;
/// Seconds per gold of a level-1 character hunting rats (2.3 gold in about 19 s).
const FRESH_SECONDS_PER_GOLD: u64 = 9;

/// A creature the character can hunt: its spawn key, health and expected gold per kill.
#[derive(Debug, Clone)]
struct Candidate {
    key: String,
    health: u64,
    milli_gold: u64,
    kill_seconds: u64,
}

impl Candidate {
    /// Wall budget for earning `needed` gold from this creature: the kills it takes.
    fn budget(&self, needed: u64) -> Duration {
        let kills = needed.saturating_mul(1000).div_ceil(self.milli_gold.max(1));
        Duration::from_secs(SETUP_SECONDS.saturating_add(kills.saturating_mul(self.kill_seconds)))
    }
}

/// What a row adds to its own outer deadline for earning `needed` gold, sized for a fresh
/// character on rats and capped; the real budget is the picked creature's own.
pub(super) fn allowance(needed: u64) -> Duration {
    Duration::from_secs(
        SETUP_SECONDS
            .saturating_add(needed.saturating_mul(FRESH_SECONDS_PER_GOLD))
            .min(ALLOWANCE_CEILING_SECONDS),
    )
}

/// The `records` of every `<dir>/<prefix>*.json`, each unwrapped to its declaration/definition.
fn records(dir: &str, prefix: &str) -> TestResult<Vec<serde_json::Value>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(dir);
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !name.starts_with(prefix) || !name.ends_with(".json") {
            continue;
        }
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        for record in document["records"].as_array().into_iter().flatten() {
            let inner = record
                .get("declaration")
                .or_else(|| record.get("definition"))
                .unwrap_or(record);
            found.push(inner.clone());
        }
    }
    Ok(found)
}

/// Creatures of the bundle's spawns (never bosses) the level can safely kill, best gold per
/// second first.
fn candidates(
    spawns: &oteryn_world_bundle::spawn::Table,
    level: u64,
) -> TestResult<Vec<Candidate>> {
    let spawned: BTreeSet<&str> = spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter())
        .filter(|point| point.boss_policy.is_none())
        .filter_map(|point| spawns.creatures.get(point.creature as usize))
        .map(|creature| creature.key.as_str())
        .collect();
    let mut health = BTreeMap::new();
    let profiles: serde_json::Value = serde_json::from_slice(&std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/test-packs/spells/r25/creature_profiles.json"),
    )?)?;
    for record in profiles["records"]
        .as_array()
        .ok_or("creature-profiles-absent")?
    {
        let (Some(key), Some(points)) = (
            record["profile"]["target"]["key"].as_str(),
            record["profile"]["data"]["profile"]["health"].as_u64(),
        ) else {
            continue;
        };
        let name = key
            .strip_prefix("canary:creature/")
            .or_else(|| key.strip_prefix("oteryn:creature."))
            .unwrap_or(key);
        health.insert(name.to_owned(), points);
    }
    let loot: BTreeMap<String, serde_json::Value> = records("content/loot", "loot-")?
        .into_iter()
        .filter_map(|loot| Some((loot["identity"]["key"].as_str()?.to_owned(), loot)))
        .collect();
    let loot_of: BTreeMap<String, String> = records("content/creatures/definitions", "creatures-")?
        .into_iter()
        .filter_map(|creature| {
            Some((
                creature["identity"]["key"].as_str()?.to_owned(),
                creature["loot"]["key"].as_str()?.to_owned(),
            ))
        })
        .collect();
    let cap = level.saturating_mul(HEALTH_PER_LEVEL).max(MIN_HEALTH_CAP);
    let mut ranked = Vec::new();
    for key in spawned {
        let Some(name) = key.strip_prefix("oteryn:creature.") else {
            continue;
        };
        let (Some(&points), Some(table)) = (
            health.get(name),
            loot_of.get(key).and_then(|loot_key| loot.get(loot_key)),
        ) else {
            continue;
        };
        if points > cap {
            continue;
        }
        let mut milli_gold = 0u128;
        for entry in table["entries"].as_array().into_iter().flatten() {
            let Some(item) = entry["item"]["key"].as_str() else {
                continue;
            };
            let Some(&(_, value)) = COINS
                .iter()
                .find(|(id, _)| item == format!("oteryn:item.tibia.i{id}"))
            else {
                continue;
            };
            let (ppm, low, high) = (
                entry["probability_ppm"].as_u64().unwrap_or(0),
                entry["min_count"].as_u64().unwrap_or(0),
                entry["max_count"].as_u64().unwrap_or(0),
            );
            milli_gold += u128::from(ppm) * u128::from(low + high) * u128::from(value) / 2000;
        }
        let milli_gold = u64::try_from(milli_gold).unwrap_or(u64::MAX);
        if milli_gold == 0 {
            continue;
        }
        ranked.push(Candidate {
            key: key.to_owned(),
            health: points,
            milli_gold,
            kill_seconds: KILL_OVERHEAD_SECONDS + points / (2 + level),
        });
    }
    // Gold per second, best first; the key keeps the order deterministic.
    ranked.sort_by(|a, b| {
        (u128::from(b.milli_gold) * u128::from(a.kill_seconds))
            .cmp(&(u128::from(a.milli_gold) * u128::from(b.kill_seconds)))
            .then_with(|| a.key.cmp(&b.key))
    });
    Ok(ranked)
}

/// The character's level from the CYCLOPEDIA general stats, as the rows query it.
async fn level(client: &mut DevClientSession) -> TestResult<u64> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[1, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || general_stats::validate(&reply.payload, general_stats::selected(client)).is_err()
        || reply.payload[0] != 1
    {
        return Err(format!("gold-level-query:{reply:?}").into());
    }
    Ok(u64::from(u32::from_le_bytes(
        reply.payload[9..13].try_into()?,
    )))
}

fn reference(id: u32) -> TestResult<NonZeroU32> {
    provisioning::item_reference(&provisioning::item_index()?, id)
}

/// Gold the character carries, from its coin stacks.
pub(super) fn coin_value(client: &DevClientSession) -> TestResult<u64> {
    let coins: Vec<(NonZeroU32, u64)> = COINS
        .iter()
        .map(|&(id, value)| reference(id).map(|key| (key, value)))
        .collect::<TestResult<_>>()?;
    Ok(client
        .inventory()
        .ok_or("gold-inventory-absent")?
        .entries
        .iter()
        .map(|entry| {
            coins
                .iter()
                .find(|(key, _)| *key == entry.item_definition_ref)
                .map_or(0, |(_, value)| u64::from(entry.count.get()) * value)
        })
        .sum())
}

/// Every `SellToPlayer` offer of the compiled trade catalogue for item `id`: (npc key, unit price).
fn offers(id: u32) -> TestResult<Vec<(String, u64)>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let item_key = format!("oteryn:item.tibia.i{id}");
    let mut found = Vec::new();
    for record in catalog["records"]
        .as_array()
        .ok_or("trade-records-absent")?
    {
        let Some(list) = record["declaration"]["offers"].as_array() else {
            continue;
        };
        let Some(seller) = record["declaration"]["identity"]["key"]
            .as_str()
            .and_then(|key| key.strip_prefix("oteryn:service.trade."))
        else {
            continue;
        };
        for offer in list {
            if offer["direction"] == "SellToPlayer" && offer["item"]["key"] == item_key {
                let price = offer["unit_price"].as_u64().ok_or("trade-price-absent")?;
                found.push((format!("oteryn:npc.{seller}"), price));
            }
        }
    }
    if found.is_empty() {
        return Err(format!("trade-catalogue-has-no-seller:item={id}").into());
    }
    Ok(found)
}

/// Gold a row must hold to buy `quantity` of item `id` from any catalogue seller (dearest seller).
pub(super) fn price(id: u32, quantity: u32) -> TestResult<u64> {
    let dearest = offers(id)?
        .into_iter()
        .map(|(_, price)| price)
        .max()
        .ok_or("trade-catalogue-has-no-seller")?;
    dearest
        .checked_mul(u64::from(quantity))
        .ok_or_else(|| "trade-price-overflow".into())
}

/// Move every coin stack still in the open corpse into the backpack.
async fn loot_coins(client: &mut DevClientSession) -> TestResult {
    let coins: Vec<NonZeroU32> = COINS
        .iter()
        .map(|&(id, _)| reference(id))
        .collect::<TestResult<_>>()?;
    for _ in 0..8 {
        let Some(handle) = client.open_container().and_then(|view| {
            view.entries
                .iter()
                .find(|entry| coins.contains(&entry.item_definition_ref))
                .map(|entry| entry.handle)
        }) else {
            return Ok(());
        };
        let before = coin_value(client)?;
        let moved = client
            .move_item(&oteryn_dev_client::ItemMoveIntent {
                source: handle,
                destination: oteryn_dev_client::ItemMoveDestination::MainBackpack,
            })
            .await?;
        if moved.outcome != items::ItemMoveOutcome::Moved {
            return Err(format!("gold-loot-move:{moved:?}").into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while coin_value(client)? <= before {
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("gold-loot-delta-missing:carried={before}").into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
        verdict::Harness::global().progress("gold-loot");
    }
    Ok(())
}

/// Hunt the best-paying reachable creature and loot its gold until the character carries
/// `needed`; returns the carried gold.
///
/// The deadline is the picked creature's own budget for the gold still missing.
pub(super) async fn earn(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    row: &str,
    needed: u64,
) -> TestResult<u64> {
    let started = tokio::time::Instant::now();
    let start_gold = coin_value(client)?;
    let level = level(client).await?;
    let ranked = candidates(spawns, level)?;
    if ranked.is_empty() && needed > start_gold {
        return Err(format!("{row}: no spawned creature pays coin gold at level {level}").into());
    }
    let mut unreachable = BTreeSet::new();
    let mut deadline = None;
    let mut kills = 0u64;
    loop {
        let observed = coin_value(client)?;
        evidence(&format!(
            "{row}_GOLD observed={observed} needed={needed} gained={} kills={kills} elapsed_s={}",
            observed.saturating_sub(start_gold),
            started.elapsed().as_secs()
        ));
        if observed >= needed {
            return Ok(observed);
        }
        if deadline.is_some_and(|limit| tokio::time::Instant::now() >= limit) {
            return Err(format!(
                "{row}: gold-earning-budget observed={observed} needed={needed} kills={kills} level={level}"
            )
            .into());
        }
        let mut picked = None;
        for (index, candidate) in ranked.iter().enumerate() {
            if unreachable.contains(&index) {
                continue;
            }
            match travel_to_spawn(client, world, spawns, &candidate.key).await {
                Ok(()) => {
                    picked = Some(candidate);
                    break;
                }
                Err(error) => {
                    evidence(&format!(
                        "{row}_GOLD skip creature={} observed={error}",
                        candidate.key
                    ));
                    unreachable.insert(index);
                }
            }
        }
        let candidate =
            picked.ok_or_else(|| format!("{row}: no reachable gold creature at level {level}"))?;
        if deadline.is_none() {
            let budget = candidate.budget(needed.saturating_sub(observed));
            deadline = Some(tokio::time::Instant::now() + budget);
            evidence(&format!(
                "{row}_GOLD target creature={} health={} expected_milli_gold_per_kill={} kill_s={} level={level} budget_s={}",
                candidate.key,
                candidate.health,
                candidate.milli_gold,
                candidate.kill_seconds,
                budget.as_secs()
            ));
        }
        let hunted = async {
            client_hunt::hunt(client).await?;
            loot_coins(client).await
        }
        .await;
        hunted.map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(
                format!("{row}: {} ATTACK/loot observed={error}", candidate.key),
                error,
            )
        })?;
        kills += 1;
        verdict::Harness::global().progress("gold");
    }
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
                .flat_map(|view| view.equipment.iter().map(|entry| &entry.item)),
        )
        .filter(|entry| entry.item_definition_ref == key)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

fn carried(client: &DevClientSession, key: NonZeroU32) -> TestResult<items::ItemHandle> {
    client
        .inventory()
        .and_then(|view| {
            view.entries
                .iter()
                .find(|entry| entry.item_definition_ref == key)
        })
        .map(|entry| entry.handle)
        .ok_or_else(|| format!("carried-item-absent:{key}").into())
}

/// Walk to the nearest catalogue seller of item `id`, TALK `hi` and `trade`, and BUY `quantity`.
pub(super) async fn buy(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    id: u32,
    quantity: u32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop;
    let key = reference(id)?;
    let sellers: BTreeSet<String> = offers(id)?.into_iter().map(|(seller, _)| seller).collect();
    let here = tile(client.world_spatial().actor_position)?;
    let (seller, path) =
        npcs.npcs
            .iter()
            .filter(|entry| sellers.contains(&entry.key))
            .filter_map(|entry| {
                route(world, here, |at| {
                    entry.placements.iter().any(|p| {
                        p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3
                    })
                })
                .map(|path| (entry, path))
            })
            .min_by_key(|(_, path)| path.len())
            .ok_or_else(|| format!("reachable-seller-absent:item={id}"))?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|view| {
            view.iter().find(|entry| {
                entry.kind == EntityKind::Npc
                    && seller.placements.iter().any(|p| {
                        entry.position.x == i32::from(p.x)
                            && entry.position.y == i32::from(p.y)
                            && entry.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
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
    let gold_before = coin_value(client)?;
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
        return Err(format!(
            "trade-buy:item={id}:reply={reply:?}:carried_gold={gold_before}:unit_price={}",
            offer.unit_price
        )
        .into());
    }
    let wanted = before + u64::from(offer.count) * u64::from(quantity);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event
                && count(client, key)? == wanted
            {
                verdict::Harness::global().progress("gold-buy");
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

/// ITEM_MOVE a carried item into an equipment slot and require the served equipment to show it.
pub(super) async fn equip(
    client: &mut DevClientSession,
    key: NonZeroU32,
    slot: EquipmentSlot,
) -> TestResult {
    let reply = client
        .move_item(&items::ItemMoveIntent {
            source: carried(client, key)?,
            destination: items::ItemMoveDestination::Equipment(slot),
        })
        .await?;
    if reply.outcome != items::ItemMoveOutcome::Moved
        || client.inventory().is_none_or(|view| {
            !view
                .equipment
                .iter()
                .any(|entry| entry.slot == slot && entry.item.item_definition_ref == key)
        })
    {
        return Err(format!("equip:item={key}:slot={slot:?}:reply={reply:?}").into());
    }
    Ok(())
}
