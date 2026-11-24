//! C12 full-row journey (Spell equipment augments), derived from the matrix, the re-audit and the
//! `item_spell_augments` owner. Every step is driven over the real client connection and every
//! effect is measured, never inferred from a mana debit or a slot.
//!
//! 1. `baseline-cast`: reach a reachable creature spawn whose content health is high enough that a
//!    spell hit is never health-capped, attack it, and CAST Greater Tiger Clash repeatedly without
//!    the item; require `Cast` and a mana debit each time and record the spell damage the server
//!    attributes to the cast (the presentation events sharing the cast's decision ordinal).
//! 2. `acquire-50271`: purchase the Fists of Enlightenment (50271) from Enpa-Deia Pema.
//! 3. `augmented-cast`: ITEM_MOVE the item to the right hand, CAST again and require the measured
//!    the cast to publish attributed damage. The weapon swap changes the attack inputs.
//! 4. `unequip`: ITEM_MOVE it back to the backpack; require the inventory delta.
//! 5. `unequipped-cast`: restore the baseline weapon and CAST with item 50271 only carried.
//! 6. `fresh-admission`: re-equip, LOGOUT, admit afresh, require the right-hand item to persist and
//!    CAST again and require the committed cast owner to prove the exact augment on its original roll.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::item_view as items;
use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationValueKind};
use std::num::NonZeroU32;

const PREPARATION: character_prep::Seeded = character_prep::Seeded {
    profile: "c12-monk",
    reason: "C12 tests spell augment behavior after obtaining the weapon and qualifying for the spell.",
};
const ITEM_ID: u32 = 50_271;
const PREREQUISITES: &str = "level-20-monk,premium,mana-potions";
const MANA_POTION: u32 = 268;
const CAST_MANA: u32 = 50;
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
            "e2e_all scenario=C12.full status={} seconds={:.3} mode=seeded prerequisites={PREREQUISITES}{detail}",
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
    character_prep::declare_seeded("C12", PREREQUISITES, "exori nia augment attribution");
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
    cast_evidence_log: std::path::PathBuf,
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
    async fn cast_damage(&self, client: &mut DevClientSession, spell: NonZeroU32, bps: u64) -> TestResult<u64> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            let victim = self.target(client).await?;
            let own = own_entity(client)?;
            let before = *client.actor_vitals().ok_or("pre-cast vitals absent")?;
            client.take_events();
            let sequence = client.last_server_sequence();
            let evidence_start = std::fs::metadata(&self.cast_evidence_log)?.len();
            let outcome = verdict::indexed_timeout(COMMAND_BUDGET, client.cast_spell(spell, SpellTarget::AttackTarget, true))
                .await
                .map_err(|_| format!("CAST deadline=8s last-frame={sequence}"))??;
            if outcome.disposition == SpellCastDisposition::CoolingDown && tokio::time::Instant::now() < deadline {
                client.service_liveness(Duration::from_millis(250)).await?;
                continue;
            }
            if outcome.status != CommandStatus::Accepted || outcome.disposition != SpellCastDisposition::Cast || outcome.actor_vitals_delta.is_none() {
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
            let session = client.join_snapshot().game_session_id;
            let marker = format!(
                "SEAM_EVIDENCE c12_base_damage session={} command={} target={} generation={} ",
                hex_bytes(session.as_bytes()), outcome.command_id, hex_bytes(&victim.identity), victim.generation,
            );
            let paired = committed_base_damage(&self.cast_evidence_log, evidence_start, &marker, bps)?;
            if paired.1 != damage {
                return Err(format!("C12 owner pair does not reconcile with served damage: base_pair={paired:?} served={damage}").into());
            }
            evidence(&format!("C12 committed_cast command={} damage={damage} base_pair={paired:?} expected_bps={bps}", outcome.command_id));
            return Ok(damage);
        }
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn committed_base_damage(
    path: &Path,
    start: u64,
    marker: &str,
    bps: u64,
) -> TestResult<(u64, u64)> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    read_base_damage(std::io::BufReader::new(file.take(512 * 1024)), marker, bps)
}

fn read_base_damage(
    reader: impl std::io::BufRead,
    marker: &str,
    bps: u64,
) -> TestResult<(u64, u64)> {
    let mut observed = None;
    for line in reader.lines() {
        let line = line?;
        let Some(fields) = line.strip_prefix(marker) else { continue };
        let (before, after) = fields.strip_prefix("before=").and_then(|s| s.split_once(" after="))
            .ok_or("malformed committed C12 damage pair")?;
        if observed.replace((before.parse::<u64>()?, after.parse::<u64>()?)).is_some() {
            return Err("multiple committed C12 damage pairs for one cast and target".into());
        }
    }
    let (before, after) = observed.ok_or_else(|| format!("committed C12 damage pair absent for {marker}"))?;
    let increase = before.checked_mul(bps).ok_or("C12 augment arithmetic overflow")? / 10_000;
    if before == 0 || (bps != 0 && increase == 0) || before.checked_add(increase) != Some(after) {
        return Err(format!("C12 exact base damage comparison failed: before={before} after={after} bps={bps}").into());
    }
    Ok((before, after))
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

async fn samples(hunt: &Hunt<'_>, client: &mut DevClientSession, spell: NonZeroU32, bps: u64) -> TestResult<Vec<u64>> {
    let mut out = Vec::new();
    for _ in 0..SAMPLES {
        restore_cast_mana(client).await?;
        let before = client.actor_vitals().ok_or("pre-sample vitals absent")?.mana;
        out.push(hunt.cast_damage(client, spell, bps).await?);
        if client.actor_vitals().ok_or("post-sample vitals absent")?.mana >= before {
            return Err(format!("Cast without a mana debit: mana {before} -> {}", client.actor_vitals().map_or(0, |v| v.mana)).into());
        }
    }
    Ok(out)
}

async fn restore_cast_mana(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    if client.actor_vitals().is_some_and(|vitals| vitals.mana >= CAST_MANA) {
        return Ok(());
    }
    let potion = provisioning::item_reference(&provisioning::item_index()?, MANA_POTION)?;
    let handle = client.inventory().and_then(|inventory| {
        inventory.entries.iter().find(|entry| entry.item_definition_ref == potion)
    }).map(|entry| entry.handle).ok_or("C12 has insufficient mana and no carried Mana Potion")?;
    let result = client.use_item(handle).await?;
    if result.status != CommandStatus::Accepted || result.disposition != UseDisposition::Committed {
        return Err(format!("Mana Potion USE refused: {result:?}").into());
    }
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    while !client.actor_vitals().is_some_and(|vitals| vitals.mana >= CAST_MANA) {
        if tokio::time::Instant::now() >= deadline {
            return Err("Mana Potion did not publish enough mana for Greater Tiger Clash".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok(())
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

fn pema_planned_price() -> TestResult<u64> {
    let trades = read_json("content/services/trade/trade-00000-00323.json")?;
    let key = format!("oteryn:item.tibia.i{ITEM_ID}");
    trades["records"].as_array().ok_or("trade records absent")?.iter()
        .map(|record| &record["declaration"])
        .filter(|declaration| declaration["identity"]["key"] == "oteryn:service.trade.enpa_deia_pema")
        .flat_map(|declaration| declaration["offers"].as_array().into_iter().flatten())
        .find(|offer| offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key)
        .and_then(|offer| offer["unit_price"].as_u64())
        .ok_or_else(|| "authored Enpa-Deia Pema price for item 50271 absent".into())
}

async fn acquire_pema(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    definition: NonZeroU32,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop;
    use oteryn_protocol_oteryn::npc_service as npc;

    let seller = npcs.npcs.iter().find(|entry| entry.key == "oteryn:npc.enpa_deia_pema")
        .ok_or("Enpa-Deia Pema absent from the compiled NPC table")?;
    let near_placement = |at: TilePos, placement: TilePos| {
        at.floor == placement.floor && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
    };
    let placements = seller.placements.iter().map(|p| TilePos { x: p.x, y: p.y, floor: p.floor });
    let route_to_seller = |from| {
        route(world, from, |at| placements.clone().any(|placement| near_placement(at, placement)))
            .ok_or("no playable route within three tiles of a compiled Enpa-Deia Pema placement")
    };
    route_to_seller(tile(client.world_spatial().actor_position)?)?;
    full_e2ef7::earn_gold(client, world, pema_planned_price()?).await?;
    let path = route_to_seller(tile(client.world_spatial().actor_position)?)?;
    walk(client, world, &path).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let actor = client.world_entities().and_then(|entities| {
        let mut matches = entities.iter().filter(|entry| {
            entry.kind == EntityKind::Npc
                && entry.display_name.is_some_and(|name| name.as_str().is_ok_and(|name| name == "Enpa-Deia Pema"))
                && placements.clone().any(|placement| {
                    near_placement(here, placement) && tile(entry.position).ok() == Some(placement)
                })
        });
        let actor = matches.next()?.entity;
        matches.next().is_none().then_some(actor)
    }).ok_or("expected one live Enpa-Deia Pema at a compiled placement")?;

    let reply = npc_shop::talk(client.session_mut(), &npc::NpcTalkIntent { npc_actor: actor, text: "hi".into() }).await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("Enpa-Deia Pema hi reply={reply:?}").into());
    }
    let greeting = client.session_mut().read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1).await?;
    let conversation = npc::decode_npc_conversation(&greeting.payload)
        .map_err(|e| format!("Pema conversation: {e:?}"))?;
    if !conversation.open.is_some_and(|(speaker, _)| speaker == actor) {
        return Err("Pema greeting did not open a conversation for the resolved actor".into());
    }
    let reply = npc_shop::talk(client.session_mut(), &npc::NpcTalkIntent { npc_actor: actor, text: "trade".into() }).await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("Enpa-Deia Pema trade reply={reply:?}").into());
    }
    let delta = client.session_mut().read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1).await?;
    let window = npc::decode_npc_trade_window(&delta.payload).map_err(|e| format!("Pema trade window: {e:?}"))?;
    let (window_actor, catalogue_revision) = window.open.ok_or("Pema trade window closed")?;
    if window_actor != actor {
        return Err("trade window opened for a different NPC".into());
    }
    let (index, offer) = window.offers.iter().enumerate()
        .find(|(_, offer)| offer.side == npc::NpcTradeSide::Buy && offer.item_definition_ref == definition.get())
        .ok_or("Pema trade window does not sell item 50271")?;
    if offer.count != 1 {
        return Err(format!("Pema offer must contain one item 50271, served {}", offer.count).into());
    }
    client.inventory().ok_or("no inventory before Pema BUY")?;
    let before = full_e2ef7::units(client, definition);
    if before != 0 {
        return Err("item 50271 already present after reload, before Pema BUY".into());
    }
    client.take_events();
    let reply = npc_shop::buy(client.session_mut(), &npc::NpcTradeIntent {
        npc_actor: actor,
        catalogue_revision,
        offer_index: u32::try_from(index)?,
        side: offer.side,
        quantity: 1,
        expected_unit_price: offer.unit_price,
    }).await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("Pema BUY reply={reply:?} price={}", offer.unit_price).into());
    }
    wait_inventory(client, "Pema BUY of item 50271", |c| {
        full_e2ef7::units(c, definition) == before + 1 && carried_count(c, definition) == 1
    }).await
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let definition = item_ref()?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_C12_MONK_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients { accounts: &accounts, characters: &characters, ..*clients };
    if !requires_platform_grant(&clients.accounts[1]) {
        return Err(super::full_row_c02::PLATFORM_ADMISSION_GAP.into());
    }
    let mut client = PREPARATION.connect(clients, 0xc2, 1).await?;
    let level = full_e2ef7::player_level(&mut client).await?;
    if level < 20 {
        return Err(format!("Platform-admitted Monk needs level 20 for item {ITEM_ID}; observed={level}").into());
    }
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    if carried_count(&client, definition) != 0 || equipped_right(&client, definition) {
        return Err("item 50271 must first enter the dedicated Monk's backpack through the purchase".into());
    }
    character_prep::grant_premium(&client).await?;
    character_prep::grant_item(&client, MANA_POTION, 12).await?;
    character_prep::reload(&mut client, clients).await?;
    acquire_pema(&mut client, world, &metadata.npcs, definition).await?;
    let spell_index = spell(&mut client)?;
    let augment = content_augment()?;
    let from = tile(client.world_spatial().actor_position)?;
    let profiles = creature_profiles()?;
    let (creature, _) = pick_creature(
        &metadata.spawns,
        |key| profiles.get(key).map(|profile| profile.health),
        |goal| { let _ = route(world, from, goal); },
        |at| world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone()),
    )?;
    let profile = profiles.get(&creature).ok_or("selected creature profile absent")?;
    let hunt = Hunt { world, spawns: &metadata.spawns, creature, profile, cast_evidence_log: required("NODE_BOOT_CAST_EVIDENCE_LOG")?.into() };

    let baseline_weapon = weapon(&client)?;
    if baseline_weapon == Some(definition) {
        return Err("need=augment-off-baseline;item-50271-already-equipped".into());
    }
    let baseline = verdict::indexed_timeout(BASELINE_BUDGET, samples(&hunt, &mut client, spell_index, 0))
        .await
        .map_err(|_| format!("baseline sampling deadline=75s last-frame={}", client.last_server_sequence()))??;
    row.record_cast("baseline-cast", &baseline, baseline_weapon);

    let acquired = carried_count(&client, definition);
    if acquired != 1 {
        return Err(format!("expected one purchased item {ITEM_ID} in backpack; observed={acquired}").into());
    }
    row.pass("acquire-50271", acquired);

    equip_right(&mut client, definition).await?;
    let augmented = samples(&hunt, &mut client, spell_index, augment).await?;
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
    let carried = samples(&hunt, &mut client, spell_index, 0).await?;
    row.record_cast("unequipped-cast", &carried, weapon(&client)?);

    equip_right(&mut client, definition).await?;
    let old_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xc3, 1).await?;
    if fresh.join_snapshot().game_session_id == old_session || !equipped_right(&fresh, definition) {
        return Err("fresh admission lost the right-hand item".into());
    }
    let fresh_spell_index = spell(&mut fresh)?;
    let replay = samples(&hunt, &mut fresh, fresh_spell_index, augment).await?;
    row.record_cast("fresh-admission", &replay, weapon(&fresh)?);
    logout(&mut fresh).await?;
    row.pass("augment-attribution", (ITEM_ID, augment));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_pair_requires_exact_unchanged_input_and_expected_delta() -> TestResult {
        const MARKER: &str = "SEAM_EVIDENCE c12_base_damage session=test command=1 target=rat generation=2 ";
        let read = |tail: &str, bps| {
            let log = format!("{MARKER}{tail}");
            read_base_damage(log.as_bytes(), MARKER, bps)
        };
        assert_eq!(read("before=37 after=40", 1_000)?, (37, 40));
        assert_eq!(read("before=37 after=37", 0)?, (37, 37));
        assert!(read("before=37 after=41", 1_000).is_err());
        assert!(read("before=37 after=37", 1_000).is_err());
        assert!(read("before=37 after=40", 0).is_err());
        let duplicate = format!("{MARKER}before=37 after=40
{MARKER}before=37 after=40");
        assert!(read_base_damage(duplicate.as_bytes(), MARKER, 1_000).is_err());
        Ok(())
    }

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
        assert!(pema_planned_price()? > 0);
        Ok(())
    }
}
