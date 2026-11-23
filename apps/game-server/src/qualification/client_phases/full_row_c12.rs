//! C12 full-row journey (Spell equipment augments), derived from the matrix, the re-audit and the
//! `item_spell_augments` owner. Every step is driven over the real client connection and every
//! effect is measured, never inferred from a mana debit or a slot.
//!
//! 1. `baseline-cast`: reach a reachable creature spawn whose content health is high enough that a
//!    spell hit is never health-capped, attack it, and CAST Greater Tiger Clash repeatedly without
//!    the item; require `Cast` and a mana debit each time and record the spell damage the server
//!    attributes to the cast (the presentation events sharing the cast's decision ordinal).
//! 2. `acquire-50271`: acquire Fists of Enlightenment (50271) in-journey through the NPC trade
//!    route the content declares (earned gold, walk, `hi`, `trade`, BUY); require the inventory
//!    delta. No route in content or no reachable seller fails with `needed=acquisition-route-for-50271`.
//! 3. `augmented-cast`: ITEM_MOVE the item to the right hand, CAST again and require the measured
//!    the cast to publish attributed damage. The weapon swap changes the attack inputs.
//! 4. `unequip`: ITEM_MOVE it back to the backpack; require the inventory delta.
//! 5. `unequipped-cast`: restore the baseline weapon and CAST with item 50271 only carried.
//! 6. `fresh-admission`: re-equip, LOGOUT, admit afresh, require the right-hand item to persist and
//!    CAST again. This observes damage and custody. Augment attribution needs independent evidence.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::item_view as items;
use oteryn_protocol_oteryn::npc_service as npc;
use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationValueKind};
use std::num::NonZeroU32;

const ITEM_ID: u32 = 50_271;
const SELLER: &str = "oteryn:npc.enpa_deia_pema";
const SPELL_WORDS: &str = "exori nia";
const SPELL_NAME: &str = "greater tiger clash";
const SAMPLES: usize = 3;
const COMMAND_BUDGET: Duration = Duration::from_secs(8);
const BASELINE_BUDGET: Duration = Duration::from_secs(75);
const STEPS: [&str; 7] = [
    "baseline-cast",
    "acquire-50271",
    "augmented-cast",
    "unequip",
    "unequipped-cast",
    "fresh-admission",
    "augment-attribution",
];

struct Row {
    started: std::time::Instant,
    passed: Vec<&'static str>,
    failure: Option<(&'static str, String)>,
}

impl Row {
    fn new() -> Self {
        Self { started: std::time::Instant::now(), passed: Vec::new(), failure: None }
    }

    fn current(&self) -> &'static str {
        STEPS.get(self.passed.len()).copied().unwrap_or("journey")
    }

    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if self.failure.is_none() && STEPS.get(self.passed.len()) == Some(&step) {
            self.passed.push(step);
            evidence(&format!("C12_CHECK {step} PASS observed={observed:?}"));
        }
    }

    fn record_cast(&mut self, step: &'static str, samples: &[u64], equipped: Option<NonZeroU32>) {
        if self.failure.is_none() && STEPS.get(self.passed.len()) == Some(&step) {
            self.passed.push(step);
            evidence(&format!("C12_OBSERVATION step={step} weapon={equipped:?} damage={samples:?}"));
        }
    }

    fn finish(mut self, result: TestResult) {
        let result: TestResult = result.and(Err(super::full_row_c02::PLATFORM_ADMISSION_GAP.into()));
        if let Err(error) = result {
            self.failure = Some((self.current(), error.to_string()));
        }
        let complete = self.failure.is_none() && self.passed.len() == STEPS.len();
        let detail = match &self.failure {
            _ if complete => String::new(),
            Some((step, observed)) => {
                let not_run = STEPS.get(self.passed.len() + 1..).unwrap_or_default();
                evidence(&format!("C12_CHECK {step} FAIL observed={observed}"));
                evidence(&gap(step, observed, not_run));
                format!(" evidence=step:{step};observed:{};not-run:{}", token(observed), not_run.join(","))
            }
            None => {
                let step = self.current();
                evidence(&gap(step, "journey ended without required check", &STEPS[self.passed.len() + 1..]));
                " evidence=journey-ended-without-every-step".to_owned()
            }
        };
        evidence(&format!(
            "e2e_all scenario=C12.full status={} seconds={:.3}{detail}",
            if complete { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

fn gap(step: &str, observed: &str, remaining: &[&str]) -> String {
    format!("C12_GAP step={step} observed={observed} not-run={}", remaining.join(","))
}

fn token(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || "-_.:/<>=".contains(c) { c } else { '-' })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = match verdict::indexed_timeout(Duration::from_secs(480), verdict::observe(journey(&mut row, clients))).await {
        Ok(result) => result,
        Err(_) => Err("journey-timeout-480s-without-server-reply".into()),
    };
    row.finish(result);
}

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(relative: &str) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&std::fs::read(root().join(relative))?)?)
}

fn spell(client: &mut DevClientSession) -> TestResult<NonZeroU32> {
    let catalog = client.session_mut().spell_catalog()?.ok_or("served spell catalogue absent")?;
    let entry = catalog.entries.iter()
        .find(|entry| entry.words == SPELL_WORDS && entry.rune_item.is_none())
        .ok_or("need=platform-created-premium-monk-or-exalted-monk;spell=exori-nia;cast-min-level=18;item-50271-min-level=20;observed=absent-from-served-spell-catalogue")?;
    if !entry.available {
        return Err(format!(
            "need=premium-monk-or-exalted-monk;spell={SPELL_WORDS};cast-min-level={};item-50271-min-level=20;learned={}",
            entry.level, entry.learned
        ).into());
    }
    NonZeroU32::new(entry.cast_index).ok_or_else(|| "Greater Tiger Clash has zero served index".into())
}

/// The base-damage augment (hundredths of a percent) that content binds to this spell on the item.
fn content_augment() -> TestResult<u64> {
    let profiles = read_json("content/items/definitions/spell-native-profiles.json")?;
    let key = format!("oteryn:item.tibia.i{ITEM_ID}");
    profiles["records"]
        .as_array()
        .ok_or("item profile records absent")?
        .iter()
        .find(|record| record["production_definition"]["production_key"] == key)
        .and_then(|record| record["attributes"]["spell_augments"].as_array())
        .and_then(|augments| {
            augments.iter().find(|a| a["kind"] == "base_damage" && a["spell"] == SPELL_NAME)
        })
        .and_then(|augment| augment["value"].as_u64())
        .ok_or_else(|| format!("content binds no base_damage augment of {SPELL_NAME} to {key}").into())
}

struct CreatureProfile {
    health: u64,
    display_name: String,
}

fn creature_profiles() -> TestResult<std::collections::BTreeMap<String, CreatureProfile>> {
    let mut profiles = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root().join("content/creatures/definitions"))? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if !(name.starts_with("creatures-") && name.ends_with(".json")) {
            continue;
        }
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        for record in document["records"].as_array().into_iter().flatten() {
            let profile = &record["authoring"]["profile"];
            if let (Some(key), Some(health), Some(name)) = (
                record["definition"]["identity"]["key"].as_str(),
                profile["health"].as_u64(),
                profile["details"]["display_name"].as_str(),
            ) {
                profiles.insert(key.to_owned(), CreatureProfile { health, display_name: name.to_owned() });
            }
        }
    }
    Ok(profiles)
}

fn item_ref() -> TestResult<NonZeroU32> {
    provisioning::item_reference(&provisioning::item_index()?, ITEM_ID)
}

fn equipped_right(client: &DevClientSession, definition: NonZeroU32) -> bool {
    client.inventory().is_some_and(|inventory| {
        inventory.equipment.iter().any(|entry| {
            entry.slot == items::EquipmentSlot::RightHand && entry.item.item_definition_ref == definition
        })
    })
}

fn carried_count(client: &DevClientSession, definition: NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == definition)
            .map(|entry| u64::from(entry.count.get()))
            .sum()
    })
}

async fn wait_inventory(
    client: &mut DevClientSession,
    what: &str,
    predicate: impl Fn(&DevClientSession) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !predicate(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no inventory delta for {what}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

fn own_entity(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let entities = client.world_entities().ok_or("own-entity-snapshot-missing")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "own-entity-missing".into())
}

fn adjacent_creature(client: &DevClientSession, display_name: &str) -> Option<oteryn_dev_client::EntityRef> {
    let at = client.world_spatial().actor_position;
    client.world_entities()?.iter().find(|entry| {
        entry.kind == EntityKind::Creature
            && entry.display_name.is_some_and(|name| name.as_str().is_ok_and(|name| name == display_name))
            && entry.position.floor == at.floor
            && entry.position.x.abs_diff(at.x).max(entry.position.y.abs_diff(at.y)) <= 1
    }).map(|entry| entry.entity)
}

/// The spawn creature whose content health is the lowest one that a spell hit cannot cap, else the
/// healthiest reachable one; the measurement gate below still refuses any possibly capped sample.
fn pick_creature(
    spawns: &oteryn_world_bundle::spawn::Table,
    health: impl Fn(&str) -> Option<u64>,
    search: impl FnOnce(&dyn Fn(TilePos) -> bool),
    combat_tile: impl Fn(TilePos) -> bool,
) -> TestResult<(String, u64)> {
    use std::cell::Cell;
    use std::collections::HashMap;

    let mut targets = HashMap::new();
    for source in &spawns.sources {
        for point in &source.points {
            let index = point.creature as usize;
            let Some(hp) = spawns.creatures.get(index).and_then(|creature| health(&creature.key)) else {
                continue;
            };
            // Prefer the lowest health >= 3,000, else the highest fallback.
            // Preserve creature-table order for equal preferred health and
            // reverse it for equal fallback health, as in the old stable sort.
            let rank = if hp >= 3_000 {
                (false, hp, index)
            } else {
                (true, u64::MAX - hp, usize::MAX - index)
            };
            let candidate = (rank, index, hp);
            for x in point.x.saturating_sub(1)..=point.x.saturating_add(1) {
                for y in point.y.saturating_sub(1)..=point.y.saturating_add(1) {
                    let at = TilePos { x, y, floor: source.floor };
                    targets.entry(at)
                        .and_modify(|best| *best = candidate.min(*best))
                        .or_insert(candidate);
                }
            }
        }
    }
    let best_possible = targets.values().min().copied().ok_or("no creature spawn has content health")?;
    let best_reached = Cell::new(None);
    // Visit each reachable tile once. Stop early only at the best possible
    // candidate, otherwise retain the best target seen when the search ends.
    search(&|at| {
        if let Some(candidate) = targets.get(&at).copied().filter(|_| combat_tile(at)) {
            best_reached.set(Some(best_reached.get().map_or(candidate, |best| candidate.min(best))));
        }
        best_reached.get() == Some(best_possible)
    });
    let (_, index, hp) = best_reached.into_inner().ok_or("no reachable creature spawn has content health")?;
    Ok((spawns.creatures[index].key.clone(), hp))
}

#[test]
fn c12_selects_spell_target_in_one_reachability_search() -> TestResult {
    use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};

    let from = TilePos { x: 25, y: 25, floor: -7 };
    let points = [(0, 25), (1, 29), (2, 27), (3, 26)]
        .map(|(creature, x)| Point {
            creature,
            x,
            y: 25,
            direction: Direction::North,
            respawn_ms: 60_000,
            boss_policy: None,
        });
    let health = [("fallback", 500), ("inaccessible", 3_000), ("preferred", 3_100), ("stronger", 4_000)];
    let spawns = Table {
        creatures: health.iter().map(|(key, _)| Creature { key: (*key).into(), period: Period::All }).collect(),
        sources: vec![Source { key: "source".into(), floor: -7, x: 25, y: 25, points: points.into() }],
    };
    let mut traversals = 0;
    let selected = pick_creature(
        &spawns,
        |key| health.iter().find_map(|(candidate, hp)| (*candidate == key).then_some(*hp)),
        |goal| {
            traversals += 1;
            let _ = find_route(from, goal, |at, step| {
                (step == StepDirection::East && at.x < 27).then_some(TilePos { x: at.x + 1, ..at })
            });
        },
        |at| at != from,
    )?;
    assert_eq!(selected, ("preferred".into(), 3_100));
    assert_eq!(traversals, 1);
    Ok(())
}

struct Hunt<'a> {
    world: &'a BundleWorld,
    spawns: &'a oteryn_world_bundle::spawn::Table,
    creature: String,
    profile: &'a CreatureProfile,
}

impl Hunt<'_> {
    async fn target(&self, client: &mut DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
        for _ in 0..4 {
            if adjacent_creature(client, &self.profile.display_name).is_none() {
                travel_to_spawn(client, self.world, self.spawns, &self.creature).await?;
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
            while adjacent_creature(client, &self.profile.display_name).is_none() && tokio::time::Instant::now() < deadline {
                client.service_liveness(Duration::from_millis(100)).await?;
                client.take_events();
            }
            if let Some(entity) = adjacent_creature(client, &self.profile.display_name) {
                let sequence = client.last_server_sequence();
                let reply = verdict::indexed_timeout(COMMAND_BUDGET, client.session_mut().attack_target(Some(&entity)))
                    .await
                    .map_err(|_| format!("ATTACK deadline=8s last-frame={sequence}"))??;
                if reply.disposition == oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
                    return Ok(entity);
                }
                return Err(format!("ATTACK reply={reply:?}").into());
            }
        }
        Err(format!("no live {} adjacent after four spawn visits", self.creature).into())
    }

    /// One cast, retried only while the server reports `CoolingDown`; returns the damage the server
    /// attributes to that cast (events sharing the decision ordinal of the caster's SpellWords).
    async fn cast_damage(&self, client: &mut DevClientSession, spell: NonZeroU32) -> TestResult<u64> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            let victim = self.target(client).await?;
            let own = own_entity(client)?;
            let before = *client.actor_vitals().ok_or("pre-cast vitals absent")?;
            client.take_events();
            let sequence = client.last_server_sequence();
            let outcome = verdict::indexed_timeout(COMMAND_BUDGET, client.cast_spell(spell, SpellTarget::AttackTarget, true))
                .await
                .map_err(|_| format!("CAST deadline=8s last-frame={sequence}"))??;
            if outcome.disposition == SpellCastDisposition::CoolingDown && tokio::time::Instant::now() < deadline {
                client.service_liveness(Duration::from_millis(250)).await?;
                continue;
            }
            if outcome.disposition != SpellCastDisposition::Cast || outcome.actor_vitals_delta.is_none() {
                return Err(format!("CAST reply={outcome:?}").into());
            }
            let mut damage = 0_u64;
            let window = tokio::time::Instant::now() + Duration::from_secs(3);
            while damage == 0 && tokio::time::Instant::now() < window {
                client.service_liveness(Duration::from_millis(50)).await?;
                damage = client.take_events().iter().map(|event| cast_damage_in(event, own, victim)).sum();
            }
            let after = *client.actor_vitals().ok_or("post-cast vitals absent")?;
            if damage == 0 {
                return Err(format!("CAST reply={outcome:?} produced no attributed spell damage frame; mana={}->{}", before.mana, after.mana).into());
            }
            if damage.saturating_mul(2) >= self.profile.health
                || client.world_entities().is_none_or(|entities| entities.get(&victim).is_none())
            {
                return Err(format!("damage={damage} may be capped by {} health={}; needed=creature-with-spell-proof-health", self.creature, self.profile.health).into());
            }
            return Ok(damage);
        }
    }
}

/// Damage to `victim` in presentation batches that also carry `own`'s SpellWords in the same decision.
fn cast_damage_in(event: &SessionEvent, own: oteryn_dev_client::EntityRef, victim: oteryn_dev_client::EntityRef) -> u64 {
    let SessionEvent::Presentation(delta) = event else { return 0 };
    let decisions: Vec<u32> = delta
        .value
        .iter()
        .filter(|e| matches!(e.kind, PresentationEventKind::SpellWords { caster, .. } if caster == own))
        .map(|e| e.decision_ordinal)
        .collect();
    delta
        .value
        .iter()
        .filter(|e| decisions.contains(&e.decision_ordinal))
        .filter_map(|e| match e.kind {
            PresentationEventKind::ValueText { kind: PresentationValueKind::DamageDealt, target, primary, .. }
                if target == victim => Some(primary.value),
            _ => None,
        })
        .sum()
}

async fn samples(hunt: &Hunt<'_>, client: &mut DevClientSession, spell: NonZeroU32) -> TestResult<Vec<u64>> {
    let mut out = Vec::new();
    for _ in 0..SAMPLES {
        let before = client.actor_vitals().ok_or("pre-sample vitals absent")?.mana;
        out.push(hunt.cast_damage(client, spell).await?);
        if client.actor_vitals().ok_or("post-sample vitals absent")?.mana >= before {
            return Err(format!("Cast without a mana debit: mana {before} -> {}", client.actor_vitals().map_or(0, |v| v.mana)).into());
        }
    }
    Ok(out)
}

async fn player_gold(client: &mut DevClientSession) -> TestResult<u64> {
    use oteryn_protocol_oteryn::atelier::resources as money;
    let reply = client
        .session_mut()
        .gated_command(money::CAPABILITY_ATELIER_RESOURCES_V1, money::COMMAND_TYPE_ATELIER_RESOURCES_QUERY, &[])
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("native player balances refused:{reply:?}").into());
    }
    let balances = money::decode_reply(&reply.payload).map_err(|e| format!("native balances:{e:?}"))?;
    balances.carried_gold.checked_add(balances.bank_gold).ok_or_else(|| "native gold overflow".into())
}

/// Acquire item 50271 through the declared NPC trade (earning the gold in play) and return nothing
/// but the confirmed inventory delta.
async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    metadata: &oteryn_world_bundle::bundle::Visited,
    definition: NonZeroU32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop;
    if !metadata.npcs.npcs.iter().any(|entry| entry.key == SELLER) {
        return Err(format!("need=50271-seller-in-served-world;seller={SELLER};placement=33603,31468,7;region=131,122").into());
    }
    let trades = read_json("content/services/trade/trade-00000-00323.json")?;
    let key = format!("oteryn:item.tibia.i{ITEM_ID}");
    let mut offers: Vec<(String, u64)> = Vec::new();
    for record in trades["records"].as_array().ok_or("trade records absent")? {
        let declaration = &record["declaration"];
        let Some(service) = declaration["identity"]["key"].as_str().and_then(|k| k.strip_prefix("oteryn:service.trade.")) else { continue };
        for offer in declaration["offers"].as_array().into_iter().flatten() {
            if let (true, Some(price)) = (
                offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key,
                offer["unit_price"].as_u64(),
            ) {
                offers.push((format!("oteryn:npc.{service}"), price));
            }
        }
    }
    let here = tile(client.world_spatial().actor_position)?;
    let reachable = |npc_key: &str, from: TilePos| {
        metadata.npcs.npcs.iter().filter(|n| n.key == npc_key).find_map(|n| {
            route(world, from, |at| n.placements.iter().any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)).map(|path| (n, path))
        })
    };
    let (seller, price) = offers
        .iter()
        .filter(|(npc_key, _)| reachable(npc_key, here).is_some())
        .min_by_key(|(_, price)| *price)
        .cloned()
        .ok_or_else(|| format!("needed=acquisition-route-for-{ITEM_ID};content-sellers={};reachable-sellers=0;loot-and-quest-routes=none-declared", offers.len()))?;
    let hunt_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(hunt_deadline, "bounded-observation");
    let mut hunts = 0_u32;
    loop {
        let have = player_gold(client).await?;
        if have >= price {
            break;
        }
        if tokio::time::Instant::now() >= hunt_deadline {
            return Err(format!("needed=player-earned-gold>={price};have={have};route=npc-trade:{seller};ordinary-hunts={hunts}-in-120s").into());
        }
        travel_to_spawn(client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(client).await?;
        hunts += 1;
    }
    let (npc_entry, path) = reachable(&seller, tile(client.world_spatial().actor_position)?)
        .ok_or("seller unreachable after the gold hunt")?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|e| {
                e.kind == EntityKind::Npc
                    && npc_entry.placements.iter().any(|p| e.position.x == i32::from(p.x) && e.position.y == i32::from(p.y) && e.position.floor == -i16::from(p.floor))
            })
        })
        .map(|e| e.entity)
        .ok_or("seller not visible as an NPC entity")?;
    for text in ["hi", "trade"] {
        let reply = npc_shop::talk(client.session_mut(), &npc::NpcTalkIntent { npc_actor: actor, text: text.into() }).await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!("NPC {text} reply={reply:?}").into());
        }
    }
    let delta = client.session_mut().read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1).await?;
    let window = npc::decode_npc_trade_window(&delta.payload).map_err(|e| format!("trade window:{e:?}"))?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| o.side == npc::NpcTradeSide::Buy && o.item_definition_ref == definition.get())
        .ok_or("served trade window does not offer the item")?;
    let before = carried_count(client, definition);
    let reply = npc_shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade window closed")?.1,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("BUY reply={reply:?} price={}", offer.unit_price).into());
    }
    wait_inventory(client, "purchased item", |c| carried_count(c, definition) > before).await?;
    Ok(client.last_server_sequence())
}

async fn equip_right(client: &mut DevClientSession, definition: NonZeroU32) -> TestResult {
    let handle = client
        .inventory()
        .and_then(|inventory| inventory.entries.iter().find(|e| e.item_definition_ref == definition))
        .map(|e| e.handle)
        .ok_or("augment item not carried")?;
    let moved = client
        .move_item(&items::ItemMoveIntent { source: handle, destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::RightHand) })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("right-hand ITEM_MOVE reply={moved:?}").into());
    }
    wait_inventory(client, "right-hand equipment", |c| equipped_right(c, definition)).await
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    loop {
        let reply = client.session_mut().logout().await?;
        if reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("logout reply={:?}", reply.result).into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
    }
}

fn weapon(client: &DevClientSession) -> TestResult<Option<NonZeroU32>> {
    Ok(client.inventory().ok_or("weapon control inventory absent")?.equipment.iter()
        .find(|entry| entry.slot == items::EquipmentSlot::RightHand)
        .map(|entry| entry.item.item_definition_ref))
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let definition = item_ref()?;
    let mut client = connect_world(clients, 0xc2).await?;
    let spell_index = spell(&mut client)?;
    let augment = content_augment()?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let from = tile(client.world_spatial().actor_position)?;
    let profiles = creature_profiles()?;
    let (creature, _) = pick_creature(
        &metadata.spawns,
        |key| profiles.get(key).map(|profile| profile.health),
        |goal| { let _ = route(world, from, goal); },
        |at| world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone()),
    )?;
    let profile = profiles.get(&creature).ok_or("selected creature profile absent")?;
    let hunt = Hunt { world, spawns: &metadata.spawns, creature, profile };

    let baseline_weapon = weapon(&client)?;
    if baseline_weapon == Some(definition) {
        return Err("need=augment-off-baseline;item-50271-already-equipped".into());
    }
    let baseline = verdict::indexed_timeout(BASELINE_BUDGET, samples(&hunt, &mut client, spell_index))
        .await
        .map_err(|_| format!("baseline sampling deadline=75s last-frame={}", client.last_server_sequence()))??;
    row.record_cast("baseline-cast", &baseline, baseline_weapon);

    let acquired = acquire(&mut client, world, &metadata, definition).await?;
    row.pass("acquire-50271", acquired);

    equip_right(&mut client, definition).await?;
    let augmented = samples(&hunt, &mut client, spell_index).await?;
    row.record_cast("augmented-cast", &augmented, weapon(&client)?);

    let equipped = client
        .inventory()
        .and_then(|i| i.equipment.iter().find(|e| e.slot == items::EquipmentSlot::RightHand && e.item.item_definition_ref == definition))
        .map(|e| e.item.handle)
        .ok_or("right-hand item absent before unequip")?;
    let moved = client
        .move_item(&items::ItemMoveIntent { source: equipped, destination: items::ItemMoveDestination::MainBackpack })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("unequip ITEM_MOVE reply={moved:?}").into());
    }
    wait_inventory(&mut client, "unequip", |c| !equipped_right(c, definition) && carried_count(c, definition) > 0).await?;
    row.pass("unequip", client.last_server_sequence());

    if let Some(baseline_weapon) = baseline_weapon {
        equip_right(&mut client, baseline_weapon).await?;
    }
    let carried = samples(&hunt, &mut client, spell_index).await?;
    row.record_cast("unequipped-cast", &carried, weapon(&client)?);

    equip_right(&mut client, definition).await?;
    let old_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xc3).await?;
    if fresh.join_snapshot().game_session_id == old_session || !equipped_right(&fresh, definition) {
        return Err("fresh admission lost the right-hand item".into());
    }
    let fresh_spell_index = spell(&mut fresh)?;
    let replay = samples(&hunt, &mut fresh, fresh_spell_index).await?;
    row.record_cast("fresh-admission", &replay, weapon(&fresh)?);
    logout(&mut fresh).await?;
    Err(format!(
        "need=augment-attribution-with-controlled-attack-inputs;item={ITEM_ID};configured-bps={augment}"
    ).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cast_observation_does_not_complete_augment_attribution() {
        let mut row = Row::new();
        for step in &STEPS[..6] {
            row.record_cast(step, &[1], None);
        }
        assert_eq!(row.current(), "augment-attribution");
    }

    #[test]
    fn c12_gap_names_the_stalled_command_and_unrun_steps() {
        let line = gap("baseline-cast", "CAST deadline", &STEPS[1..]);
        assert!(line.starts_with("C12_GAP step=baseline-cast observed=CAST deadline"));
        assert!(line.contains("not-run=acquire-50271,augmented-cast,unequip,unequipped-cast,fresh-admission,augment-attribution"));
    }

    #[test]
    fn pass_requires_every_step_in_order() {
        let mut row = Row::new();
        row.pass("acquire-50271", "out of order");
        assert!(row.passed.is_empty());
        for step in STEPS {
            row.pass(step, "ok");
        }
        assert_eq!(row.passed.len(), STEPS.len());
    }

    #[test]
    fn content_binds_the_augment_to_the_item() -> TestResult {
        assert!(content_augment()? > 0);
        assert_ne!(item_ref()?.get(), 0);
        Ok(())
    }

    #[test]
    fn the_declared_trade_sells_the_item() -> TestResult {
        let trades = read_json("content/services/trade/trade-00000-00323.json")?;
        let key = format!("oteryn:item.tibia.i{ITEM_ID}");
        assert!(trades["records"].as_array().ok_or("records")?.iter().any(|record| {
            record["declaration"]["offers"].as_array().is_some_and(|offers| {
                offers.iter().any(|o| o["direction"] == "SellToPlayer" && o["item"]["key"] == key)
            })
        }));
        Ok(())
    }
}
