//! C12 full-row journey (Spell equipment augments), derived from the matrix, the re-audit and the
//! `item_spell_augments` owner. Every step is driven over the real client connection and every
//! effect is measured, never inferred from a mana debit or a slot.
//!
//! 1. `acquire-50271`: purchase the Fists of Enlightenment (50271) from Enpa-Deia Pema.
//! 2. `baseline-cast`: reach a reachable creature spawn whose content health is high enough that a
//!    spell hit is never health-capped, attack it, and CAST Greater Tiger Clash repeatedly without
//!    the item; require `Cast` and a mana debit each time and record the spell damage the server
//!    attributes to the cast (the presentation events sharing the cast's decision ordinal).
//! 3. `augmented-cast`: ITEM_MOVE the item to the right hand, CAST again and require attributed
//!    damage on the original damage roll. The weapon swap changes the attack inputs.
//! 4. `unequip`: ITEM_MOVE it back to the backpack; require the inventory delta.
//! 5. `unequipped-cast`: restore the baseline weapon and CAST with item 50271 only carried.
//! 6. `fresh-admission`: re-equip, LOGOUT, admit afresh, require the right-hand item to persist and
//!    CAST again and require the committed cast owner to prove the exact augment on its original roll.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{ActorVitals, SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::item_view as items;
use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationValueKind};
use std::num::NonZeroU32;

const PREPARATION: character_prep::Seeded = character_prep::Seeded {
    profile: "c12-monk",
    reason: "C12 tests spell augment behavior after obtaining the weapon and qualifying for the spell.",
};
const ITEM_ID: u32 = 50_271;
const AUGMENT_BPS: u64 = 800;
const PEMA_PRICE: u64 = 20_000;
const PREREQUISITES: &str = "level-20-monk,premium,purchase-gold,mana-potions";
const CRYSTAL_COIN: u32 = 3043;
const CRYSTAL_COIN_VALUE: u64 = 10_000;
const MANA_POTION: u32 = 268;
const CAST_MANA: u32 = 50;
const SPELL_WORDS: &str = "exori nia";
const SPELL_NAME: &str = "greater tiger clash";
const SAMPLES: usize = 3;
const COMMAND_BUDGET: Duration = Duration::from_secs(8);
const BASELINE_BUDGET: Duration = Duration::from_secs(75);
// The authored Blue Valley round trip adds paced steps before the four cast phases.
const JOURNEY_BUDGET: Duration = Duration::from_secs(720);
const STEPS: [&str; 7] = [
    "acquire-50271",
    "baseline-cast",
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
        Self {
            started: std::time::Instant::now(),
            passed: Vec::new(),
            failure: None,
        }
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
            evidence(&format!(
                "C12_OBSERVATION step={step} weapon={equipped:?} damage={samples:?}"
            ));
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
                format!(
                    " evidence=step:{step};observed:{};not-run:{}",
                    token(observed),
                    not_run.join(",")
                )
            }
            None => {
                let step = self.current();
                evidence(&gap(
                    step,
                    "journey ended without required check",
                    &STEPS[self.passed.len() + 1..],
                ));
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
    format!(
        "C12_GAP step={step} observed={observed} not-run={}",
        remaining.join(",")
    )
}

fn token(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.:/<>=".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded("C12", PREREQUISITES, "exori nia augment attribution");
    let mut row = Row::new();
    let result = match verdict::indexed_timeout(
        JOURNEY_BUDGET,
        verdict::observe(journey(&mut row, clients)),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(format!("journey-timeout-{}s", JOURNEY_BUDGET.as_secs()).into()),
    };
    row.finish(result);
}

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(relative: &str) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&std::fs::read(
        root().join(relative),
    )?)?)
}

fn spell(client: &mut DevClientSession) -> TestResult<NonZeroU32> {
    let catalog = client
        .session_mut()
        .spell_catalog()?
        .ok_or("served spell catalogue absent")?;
    let entry = catalog.entries.iter()
        .find(|entry| entry.words == SPELL_WORDS && entry.rune_item.is_none())
        .ok_or("need=platform-created-premium-monk-or-exalted-monk;spell=exori-nia;cast-min-level=18;item-50271-min-level=20;observed=absent-from-served-spell-catalogue")?;
    if !entry.available {
        return Err(format!(
            "need=premium-monk-or-exalted-monk;spell={SPELL_WORDS};cast-min-level={};item-50271-min-level=20;learned={}",
            entry.level, entry.learned
        ).into());
    }
    NonZeroU32::new(entry.cast_index)
        .ok_or_else(|| "Greater Tiger Clash has zero served index".into())
}

/// The base-damage augment (hundredths of a percent) that content binds to this spell on the item.
pub(super) fn content_augment() -> TestResult<u64> {
    let profiles = read_json("content/items/definitions/spell-native-profiles.json")?;
    let key = format!("oteryn:item.tibia.i{ITEM_ID}");
    let authored = profiles["records"]
        .as_array()
        .ok_or("item profile records absent")?
        .iter()
        .find(|record| record["production_definition"]["production_key"] == key)
        .and_then(|record| record["attributes"]["spell_augments"].as_array())
        .and_then(|augments| {
            augments
                .iter()
                .find(|a| a["kind"] == "base_damage" && a["spell"] == SPELL_NAME)
        })
        .and_then(|augment| augment["value"].as_u64())
        .ok_or_else(|| format!("content binds no base_damage augment of {SPELL_NAME} to {key}"))?;
    if authored != AUGMENT_BPS {
        return Err(format!("C12 expected {AUGMENT_BPS} basis points, authored {authored}").into());
    }
    Ok(authored)
}

struct CreatureProfile {
    health: u64,
    display_name: String,
}

fn creature_profiles() -> TestResult<std::collections::BTreeMap<String, CreatureProfile>> {
    let mut profiles = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root().join("content/creatures/definitions"))? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
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
                profiles.insert(
                    key.to_owned(),
                    CreatureProfile {
                        health,
                        display_name: name.to_owned(),
                    },
                );
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
            entry.slot == items::EquipmentSlot::RightHand
                && entry.item.item_definition_ref == definition
                && entry.item.count.get() == 1
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
    let entities = client
        .world_entities()
        .ok_or("own-entity-snapshot-missing")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "own-entity-missing".into())
}

fn adjacent_creature(
    client: &DevClientSession,
    display_name: &str,
) -> Option<oteryn_dev_client::EntityRef> {
    let at = client.world_spatial().actor_position;
    client
        .world_entities()?
        .iter()
        .find(|entry| {
            entry.kind == EntityKind::Creature
                && entry
                    .display_name
                    .is_some_and(|name| name.as_str().is_ok_and(|name| name == display_name))
                && entry.position.floor == at.floor
                && entry
                    .position
                    .x
                    .abs_diff(at.x)
                    .max(entry.position.y.abs_diff(at.y))
                    <= 1
        })
        .map(|entry| entry.entity)
}

/// Select the reachable creature with the lowest content health of at least 3,000.
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
            let Some(hp) = spawns
                .creatures
                .get(index)
                .and_then(|creature| health(&creature.key))
            else {
                continue;
            };
            if hp < 3_000 {
                continue;
            }
            let candidate = (hp, index);
            for x in point.x.saturating_sub(1)..=point.x.saturating_add(1) {
                for y in point.y.saturating_sub(1)..=point.y.saturating_add(1) {
                    let at = TilePos {
                        x,
                        y,
                        floor: source.floor,
                    };
                    targets
                        .entry(at)
                        .and_modify(|best| *best = candidate.min(*best))
                        .or_insert(candidate);
                }
            }
        }
    }
    let best_possible = targets
        .values()
        .min()
        .copied()
        .ok_or("no creature spawn has content health")?;
    let best_reached = Cell::new(None);
    // Visit each reachable tile once. Stop early only at the best possible
    // candidate, otherwise retain the best target seen when the search ends.
    search(&|at| {
        if let Some(candidate) = targets.get(&at).copied().filter(|_| combat_tile(at)) {
            best_reached.set(Some(
                best_reached
                    .get()
                    .map_or(candidate, |best| candidate.min(best)),
            ));
        }
        best_reached.get() == Some(best_possible)
    });
    let (hp, index) = best_reached
        .into_inner()
        .ok_or("no reachable creature spawn has content health")?;
    Ok((spawns.creatures[index].key.clone(), hp))
}

#[test]
fn c12_selects_spell_target_in_one_reachability_search() -> TestResult {
    use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};

    let from = TilePos {
        x: 25,
        y: 25,
        floor: -7,
    };
    let points = [(0, 25), (1, 29), (2, 27), (3, 26)].map(|(creature, x)| Point {
        creature,
        x,
        y: 25,
        direction: Direction::North,
        respawn_ms: 60_000,
        boss_policy: None,
    });
    let health = [
        ("fallback", 500),
        ("inaccessible", 3_000),
        ("preferred", 3_100),
        ("stronger", 4_000),
    ];
    let spawns = Table {
        creatures: health
            .iter()
            .map(|(key, _)| Creature {
                key: (*key).into(),
                period: Period::All,
            })
            .collect(),
        sources: vec![Source {
            key: "source".into(),
            floor: -7,
            x: 25,
            y: 25,
            points: points.into(),
        }],
    };
    let mut traversals = 0;
    let selected = pick_creature(
        &spawns,
        |key| {
            health
                .iter()
                .find_map(|(candidate, hp)| (*candidate == key).then_some(*hp))
        },
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
    async fn approach(&self, client: &mut DevClientSession) -> TestResult {
        let from = tile(client.world_spatial().actor_position)?;
        let path = route(self.world, from, |at| {
            self.world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
                && self.spawns.sources.iter().any(|source| {
                    at.floor == source.floor
                        && source.points.iter().any(|point| {
                            at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) == 1
                                && self
                                    .spawns
                                    .creatures
                                    .get(point.creature as usize)
                                    .is_some_and(|entry| entry.key == self.creature)
                        })
                })
        })
        .ok_or("no route to the selected C12 creature spawn")?;
        walk_exact_route(client, self.world, &path).await
    }

    async fn wait_visible_target(
        &self,
        client: &mut DevClientSession,
    ) -> TestResult<oteryn_dev_client::EntityRef> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(entity) = adjacent_creature(client, &self.profile.display_name) {
                return Ok(entity);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(
                    format!("no live {} adjacent at the selected spawn", self.creature).into(),
                );
            }
            client.service_liveness(Duration::from_millis(100)).await?;
            client.take_events();
        }
    }

    async fn target(
        &self,
        client: &mut DevClientSession,
    ) -> TestResult<oteryn_dev_client::EntityRef> {
        let entity = self.wait_visible_target(client).await?;
        if combat_target(client)? != Some(entity) {
            let sequence = client.last_server_sequence();
            let reply = verdict::indexed_timeout(
                COMMAND_BUDGET,
                client.session_mut().attack_target(Some(&entity)),
            )
            .await
            .map_err(|_| format!("ATTACK deadline=8s last-frame={sequence}"))??;
            require_mechanic_acceptance(reply.status, "C12 ATTACK")?;
            if reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
                return Err(format!("ATTACK reply={reply:?}").into());
            }
            wait_combat_target(client, Some(entity)).await?;
        }
        Ok(entity)
    }

    /// Verify damage attributed to the cast and the vitals applied by the client.
    async fn cast_damage(
        &self,
        client: &mut DevClientSession,
        spell: NonZeroU32,
        bps: u64,
    ) -> TestResult<(u64, ActorVitals)> {
        let victim = self.target(client).await?;
        let own = own_entity(client)?;
        let before = *client.actor_vitals().ok_or("pre-cast vitals absent")?;
        client.take_events();
        let sequence = client.last_server_sequence();
        let evidence_start = std::fs::metadata(&self.cast_evidence_log)?.len();
        let outcome = verdict::indexed_timeout(
            COMMAND_BUDGET,
            client.cast_spell(spell, SpellTarget::AttackTarget, true),
        )
        .await
        .map_err(|_| format!("CAST deadline=8s last-frame={sequence}"))??;
        require_mechanic_acceptance(outcome.status, "C12 CAST")?;
        if outcome.disposition != SpellCastDisposition::Cast {
            return Err(format!("CAST reply={outcome:?}").into());
        }
        let vitals = outcome
            .actor_vitals_delta
            .as_ref()
            .ok_or("C12 accepted cast omitted actor-vitals")?
            .value;
        if before.mana.checked_sub(CAST_MANA) != Some(vitals.mana) {
            return Err(format!(
                "C12 expected {CAST_MANA} mana spent, observed {} -> {}",
                before.mana, vitals.mana
            )
            .into());
        }
        if client.actor_vitals() != Some(&vitals) {
            return Err("C12 cast vitals were not applied to the client view".into());
        }
        let mut damage = 0_u64;
        let window = tokio::time::Instant::now() + Duration::from_secs(3);
        while damage == 0 && tokio::time::Instant::now() < window {
            client.service_liveness(Duration::from_millis(50)).await?;
            damage = client
                .take_events()
                .iter()
                .map(|event| cast_damage_in(event, own, victim))
                .sum();
        }
        let after = *client.actor_vitals().ok_or("post-cast vitals absent")?;
        if damage == 0 {
            return Err(format!(
                "CAST reply={outcome:?} produced no attributed spell damage frame; mana={}->{}",
                before.mana, after.mana
            )
            .into());
        }
        if damage.saturating_mul(2) >= self.profile.health
            || client
                .world_entities()
                .is_none_or(|entities| entities.get(&victim).is_none())
        {
            return Err(format!("damage={damage} may be capped by {} health={}; needed=creature-with-spell-proof-health", self.creature, self.profile.health).into());
        }
        let session = client.join_snapshot().game_session_id;
        let marker = format!(
            "SEAM_EVIDENCE c12_base_damage session={} command={} target={} generation={} ",
            hex_bytes(session.as_bytes()),
            outcome.command_id,
            hex_bytes(&victim.identity),
            victim.generation,
        );
        let paired = committed_base_damage(&self.cast_evidence_log, evidence_start, &marker, bps)?;
        if paired.1 != damage {
            return Err(format!("C12 owner pair does not reconcile with served damage: base_pair={paired:?} served={damage}").into());
        }
        evidence(&format!(
            "C12 committed_cast command={} damage={damage} base_pair={paired:?} expected_bps={bps}",
            outcome.command_id
        ));
        Ok((damage, vitals))
    }
}

pub(super) fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn committed_base_damage(
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
        let Some(fields) = line.strip_prefix(marker) else {
            continue;
        };
        let (before, after) = fields
            .strip_prefix("before=")
            .and_then(|s| s.split_once(" after="))
            .ok_or("malformed committed C12 damage pair")?;
        if observed
            .replace((before.parse::<u64>()?, after.parse::<u64>()?))
            .is_some()
        {
            return Err("multiple committed C12 damage pairs for one cast and target".into());
        }
    }
    let (before, after) =
        observed.ok_or_else(|| format!("committed C12 damage pair absent for {marker}"))?;
    let increase = before
        .checked_mul(bps)
        .ok_or("C12 augment arithmetic overflow")?
        / 10_000;
    if before == 0 || (bps != 0 && increase == 0) || before.checked_add(increase) != Some(after) {
        return Err(format!(
            "C12 exact base damage comparison failed: before={before} after={after} bps={bps}"
        )
        .into());
    }
    Ok((before, after))
}

/// Damage to `victim` in presentation batches that also carry `own`'s SpellWords in the same decision.
pub(super) fn cast_damage_in(
    event: &SessionEvent,
    own: oteryn_dev_client::EntityRef,
    victim: oteryn_dev_client::EntityRef,
) -> u64 {
    let SessionEvent::Presentation(delta) = event else {
        return 0;
    };
    let decisions: Vec<u32> = delta
        .value
        .iter()
        .filter(
            |e| matches!(e.kind, PresentationEventKind::SpellWords { caster, .. } if caster == own),
        )
        .map(|e| e.decision_ordinal)
        .collect();
    delta
        .value
        .iter()
        .filter(|e| decisions.contains(&e.decision_ordinal))
        .filter_map(|e| match e.kind {
            PresentationEventKind::ValueText {
                kind: PresentationValueKind::DamageDealt,
                target,
                primary,
                ..
            } if target == victim => Some(primary.value),
            _ => None,
        })
        .sum()
}

async fn samples(
    hunt: &Hunt<'_>,
    client: &mut DevClientSession,
    spell: NonZeroU32,
    bps: u64,
) -> TestResult<Vec<u64>> {
    let mut out = Vec::new();
    for _ in 0..SAMPLES {
        restore_cast_mana(client).await?;
        let before = client
            .actor_vitals()
            .ok_or("pre-sample vitals absent")?
            .mana;
        out.push(hunt.cast_damage(client, spell, bps).await?.0);
        if client
            .actor_vitals()
            .ok_or("post-sample vitals absent")?
            .mana
            >= before
        {
            return Err(format!(
                "Cast without a mana debit: mana {before} -> {}",
                client.actor_vitals().map_or(0, |v| v.mana)
            )
            .into());
        }
        // Greater Tiger Clash has an authored 8 s spell cooldown. Let the successful
        // cast finish cooling down before the next sample or equipment phase.
        client.service_liveness(Duration::from_secs(8)).await?;
    }
    Ok(out)
}

async fn restore_cast_mana(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    if client
        .actor_vitals()
        .is_some_and(|vitals| vitals.mana >= CAST_MANA)
    {
        return Ok(());
    }
    let potion = provisioning::item_reference(&provisioning::item_index()?, MANA_POTION)?;
    let before = carried_count(client, potion);
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == potion)
        })
        .map(|entry| entry.handle)
        .ok_or("C12 has insufficient mana and no carried Mana Potion")?;
    let result = client.use_item(handle).await?;
    require_mechanic_acceptance(result.status, "C12 Mana Potion USE")?;
    if result.disposition != UseDisposition::Committed {
        return Err(format!("Mana Potion USE refused: {result:?}").into());
    }
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    while !client
        .actor_vitals()
        .is_some_and(|vitals| vitals.mana >= CAST_MANA)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("Mana Potion did not publish enough mana for Greater Tiger Clash".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let expected = before
        .checked_sub(1)
        .ok_or("C12 Mana Potion pre-state was empty")?;
    wait_inventory(client, "Mana Potion consumption", |client| {
        carried_count(client, potion) == expected
    })
    .await
}

async fn equip_right(client: &mut DevClientSession, definition: NonZeroU32) -> TestResult {
    let carried = carried_count(client, definition);
    if equipped_right(client, definition) || carried == 0 {
        return Err("right-hand equip requires a carried item and a different pre-state".into());
    }
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|e| e.item_definition_ref == definition)
        })
        .map(|e| e.handle)
        .ok_or("augment item not carried")?;
    let moved = client
        .session_mut()
        .move_item_counted(
            oteryn_dev_client::native_session::item_movement::MoveRequest {
                source: handle,
                destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::RightHand),
                count: None,
            },
        )
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("right-hand ITEM_MOVE reply={moved:?}").into());
    }
    wait_inventory(client, "right-hand equipment", |c| {
        equipped_right(c, definition) && carried_count(c, definition) == carried - 1
    })
    .await
}

fn combat_target(
    client: &mut DevClientSession,
) -> TestResult<Option<oteryn_dev_client::EntityRef>> {
    Ok(client
        .session_mut()
        .combat_state()
        .ok_or("C12 combat state absent")?
        .target)
}

async fn wait_combat_target(
    client: &mut DevClientSession,
    expected: Option<oteryn_dev_client::EntityRef>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    while combat_target(client)? != expected {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C12 combat target did not become {expected:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    let state = client
        .session_mut()
        .combat_state()
        .ok_or("C12 combat state absent before logout")?;
    if state.target.is_some() {
        let reply = client.session_mut().attack_target(None).await?;
        require_mechanic_acceptance(reply.status, "C12 clear attack")?;
        if reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
            return Err(format!("C12 clear attack before logout refused: {reply:?}").into());
        }
        wait_combat_target(client, None).await?;
    }
    while client
        .session_mut()
        .combat_state()
        .is_some_and(|state| state.in_fight)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("C12 combat lock did not expire before logout".into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
    }
    let (result, detail, _, _) = logout_once(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C12 logout refused on first attempt: {detail}").into());
    }
    Ok(())
}

fn weapon(client: &DevClientSession) -> TestResult<Option<NonZeroU32>> {
    Ok(client
        .inventory()
        .ok_or("weapon control inventory absent")?
        .equipment
        .iter()
        .find(|entry| entry.slot == items::EquipmentSlot::RightHand)
        .map(|entry| entry.item.item_definition_ref))
}

fn pema_planned_price() -> TestResult<u64> {
    let trades = read_json("content/services/trade/trade-00000-00323.json")?;
    let key = format!("oteryn:item.tibia.i{ITEM_ID}");
    let authored = trades["records"]
        .as_array()
        .ok_or("trade records absent")?
        .iter()
        .map(|record| &record["declaration"])
        .filter(|declaration| {
            declaration["identity"]["key"] == "oteryn:service.trade.enpa_deia_pema"
        })
        .flat_map(|declaration| declaration["offers"].as_array().into_iter().flatten())
        .find(|offer| offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key)
        .and_then(|offer| offer["unit_price"].as_u64())
        .ok_or("authored Enpa-Deia Pema price for item 50271 absent")?;
    if authored != PEMA_PRICE {
        return Err(format!("C12 expected Pema price {PEMA_PRICE}, authored {authored}").into());
    }
    Ok(authored)
}

async fn walk_exact_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, from, direction).ok_or("C12 planned STEP has no landing")?;
        if expected == from {
            return Err("C12 planned STEP does not change position".into());
        }
        let step = client.step(direction).await?;
        require_mechanic_acceptance(step.status, "C12 STEP")?;
        let actual = tile(client.world_spatial().actor_position)?;
        if step.disposition != StepDisposition::Moved
            || step.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!(
                "C12 purchase STEP {direction:?} from={from:?} expected={expected:?} actual={actual:?}: {step:?}"
            ).into());
        }
        verdict::Harness::global().progress("walk");
        client
            .session_mut()
            .service_liveness_discarding_events(Duration::from_millis(u64::from(step.step_wait_ms)))
            .await?;
    }
    Ok(())
}

async fn use_blue_valley_portal(
    client: &mut DevClientSession,
    world: &BundleWorld,
    source: TilePos,
    ordinal: u8,
    landing: TilePos,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == source.floor && at.x.abs_diff(source.x).max(at.y.abs_diff(source.y)) <= 1
    })
    .ok_or_else(|| format!("C12 no playable route to Blue Valley portal {source:?}"))?;
    walk_exact_route(client, world, &path).await?;
    if tile(client.world_spatial().actor_position)? == landing {
        return Err("C12 actor was already on the portal landing before USE".into());
    }
    let key = crate::map::view::placement_key(source, ordinal)
        .ok_or("C12 Blue Valley portal placement key absent")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let reply = client.use_object(&placement, 0).await?;
    require_mechanic_acceptance(reply.status, "C12 Blue Valley portal USE")?;
    if reply.disposition != UseDisposition::Committed {
        return Err(format!("C12 Blue Valley portal {source:?} USE reply={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    while tile(client.world_spatial().actor_position)? != landing {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C12 Blue Valley portal {source:?} expected landing {landing:?}, observed {:?}",
                client.world_spatial().actor_position
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    evidence(&format!("C12_PORTAL source={source:?} landing={landing:?}"));
    Ok(())
}

async fn acquire_pema(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    definition: NonZeroU32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop;
    use oteryn_protocol_oteryn::npc_service as npc;

    let seller = npcs
        .npcs
        .iter()
        .find(|entry| entry.key == "oteryn:npc.enpa_deia_pema")
        .ok_or("Enpa-Deia Pema absent from the compiled NPC table")?;
    let near_placement = |at: TilePos, placement: TilePos| {
        at.floor == placement.floor
            && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
    };
    let placements = seller.placements.iter().map(|p| TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    });
    let route_to_seller = |from| {
        route(world, from, |at| {
            placements
                .clone()
                .any(|placement| near_placement(at, placement))
        })
        .ok_or("no playable route within three tiles of a compiled Enpa-Deia Pema placement")
    };
    // Crystal 201189f8 actions_blue_valley_teleports.lua:4 is the mainland ingress.
    use_blue_valley_portal(
        client,
        world,
        TilePos {
            x: 32456,
            y: 32290,
            floor: -7,
        },
        1,
        TilePos {
            x: 33614,
            y: 31494,
            floor: -7,
        },
    )
    .await?;
    let path = route_to_seller(tile(client.world_spatial().actor_position)?)?;
    walk_exact_route(client, world, &path).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            let mut matches = entities.iter().filter(|entry| {
                entry.kind == EntityKind::Npc
                    && entry.display_name.is_some_and(|name| {
                        name.as_str().is_ok_and(|name| name == "Enpa-Deia Pema")
                    })
                    && placements.clone().any(|placement| {
                        near_placement(here, placement)
                            && tile(entry.position).ok() == Some(placement)
                    })
            });
            let actor = matches.next()?.entity;
            matches.next().is_none().then_some(actor)
        })
        .ok_or_else(|| {
            let visible: Vec<_> = client
                .world_entities()
                .into_iter()
                .flat_map(|entities| entities.iter())
                .filter(|entry| entry.kind == EntityKind::Npc)
                .map(|entry| (entry.display_name, entry.position))
                .collect();
            format!(
                "expected one live Enpa-Deia Pema at a compiled placement; visible NPCs={visible:?}"
            )
        })?;

    let conversation_before = client
        .session_mut()
        .gated_snapshot(npc::STATE_DOMAIN_NPC_CONVERSATION)
        .ok_or("Pema conversation pre-state absent")?;
    if npc::decode_npc_conversation(&conversation_before.payload)
        .map_err(|error| format!("Pema conversation pre-state: {error:?}"))?
        .open
        .is_some()
    {
        return Err("Pema conversation was already open before greeting".into());
    }
    let reply = npc_shop::talk(
        client.session_mut(),
        &npc::NpcTalkIntent {
            npc_actor: actor,
            text: "hi".into(),
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("Enpa-Deia Pema hi reply={reply:?}").into());
    }
    let greeting = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    let conversation = npc::decode_npc_conversation(&greeting.payload)
        .map_err(|e| format!("Pema conversation: {e:?}"))?;
    if !conversation
        .open
        .is_some_and(|(speaker, _)| speaker == actor)
    {
        return Err("Pema greeting did not open a conversation for the resolved actor".into());
    }
    let details = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
    let trade_before = client
        .session_mut()
        .gated_snapshot(npc::STATE_DOMAIN_NPC_TRADE_WINDOW)
        .ok_or("Pema trade window pre-state absent")?;
    if npc::decode_npc_trade_window_for(&trade_before.payload, details)
        .map_err(|error| format!("Pema trade window pre-state: {error:?}"))?
        .open
        .is_some()
    {
        return Err("Pema trade window was already open before trade".into());
    }
    let reply = npc_shop::talk(
        client.session_mut(),
        &npc::NpcTalkIntent {
            npc_actor: actor,
            text: "trade".into(),
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("Enpa-Deia Pema trade reply={reply:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = npc::decode_npc_trade_window_for(&delta.payload, details)
        .map_err(|e| format!("Pema trade window: {e:?}"))?;
    let (window_actor, catalogue_revision) = window.open.ok_or("Pema trade window closed")?;
    if window_actor != actor {
        return Err("trade window opened for a different NPC".into());
    }
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == npc::NpcTradeSide::Buy && offer.item_definition_ref == definition.get()
        })
        .ok_or("Pema trade window does not sell item 50271")?;
    if offer.count != 1 {
        return Err(format!(
            "Pema offer must contain one item 50271, served {}",
            offer.count
        )
        .into());
    }
    let price = pema_planned_price()?;
    if u64::from(offer.unit_price) != price {
        return Err(format!(
            "Pema offer expected authored price {price}, served {}",
            offer.unit_price
        )
        .into());
    }
    client.inventory().ok_or("no inventory before Pema BUY")?;
    let before = full_e2ef7::units(client, definition);
    if before != 0 {
        return Err("item 50271 already present after reload, before Pema BUY".into());
    }
    let gold_before = full_e2ef7::player_gold(client).await?;
    client.take_events();
    let reply = npc_shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("Pema BUY reply={reply:?} price={}", offer.unit_price).into());
    }
    wait_inventory(client, "Pema BUY of item 50271", |c| {
        full_e2ef7::units(c, definition) == before + 1 && carried_count(c, definition) == 1
    })
    .await?;
    let gold_after = full_e2ef7::player_gold(client).await?;
    if gold_before.checked_sub(price) != Some(gold_after) {
        return Err(format!(
            "Pema BUY expected charge {price}, observed {gold_before} -> {gold_after}"
        )
        .into());
    }
    // Crystal's return portal next to Pema leads back to the mainland hunt.
    use_blue_valley_portal(
        client,
        world,
        TilePos {
            x: 33605,
            y: 31468,
            floor: -7,
        },
        3,
        TilePos {
            x: 32456,
            y: 32291,
            floor: -7,
        },
    )
    .await?;
    Ok(gold_after)
}

fn select_hunt<'a>(
    client: &DevClientSession,
    world: &'a BundleWorld,
    spawns: &'a oteryn_world_bundle::spawn::Table,
    profiles: &'a std::collections::BTreeMap<String, CreatureProfile>,
) -> TestResult<Hunt<'a>> {
    let from = tile(client.world_spatial().actor_position)?;
    let (creature, _) = pick_creature(
        spawns,
        |key| profiles.get(key).map(|profile| profile.health),
        |goal| {
            let _ = route(world, from, goal);
        },
        |at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
        },
    )?;
    let profile = profiles
        .get(&creature)
        .ok_or("selected creature profile absent")?;
    Ok(Hunt {
        world,
        spawns,
        creature,
        profile,
        cast_evidence_log: required("NODE_BOOT_CAST_EVIDENCE_LOG")?.into(),
    })
}

async fn fresh_c12_admission(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let previous = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let fresh = connect_character(clients, 0xc3, 1).await?;
    if fresh.join_snapshot().game_session_id == previous {
        return Err("C12 preparation reused its previous session".into());
    }
    Ok(fresh)
}

async fn prepare_c12_monk(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    evidence(&format!(
        "character_preparation mode=Seeded profile={} reason={:?}",
        PREPARATION.profile, PREPARATION.reason
    ));
    let client = connect_character(clients, 0xc2, 1).await?;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        PREPARATION.profile.into(),
    )
    .await?;
    fresh_c12_admission(client, clients).await
}

async fn verify_seeded_monk(client: &mut DevClientSession) -> TestResult {
    let level = full_e2ef7::player_level(client).await?;
    let vitals = client
        .actor_vitals()
        .ok_or("C12 seeded Monk has no actor-vitals snapshot")?;
    // Crystal's level 8 pools are 185 HP and 90 mana. Monk adds 10 to each per level.
    if (level, vitals.max_health, vitals.max_mana) != (20, 305, 210) {
        return Err(format!(
            "C12 expected level 20 Monk with 305 max HP and 210 max mana, observed level={level} vitals={vitals:?}"
        ).into());
    }
    Ok(())
}

pub(super) async fn require_current_premium<S: oteryn_dev_client::native_session::SessionStream>(
    session: &mut oteryn_dev_client::native_session::Session<S>,
) -> TestResult {
    use oteryn_protocol_oteryn::{character_info, premium_overview};
    let reply = session
        .gated_command(
            character_info::CAPABILITY_CHARACTER_INFO_V1,
            character_info::COMMAND_TYPE_CHARACTER_INFO,
            &[premium_overview::OPERATION],
        )
        .await?;
    require_mechanic_acceptance(reply.status, "C12 Premium overview")?;
    verdict::Harness::global().progress("premium");
    let overview = premium_overview::Overview::decode(&reply.payload)
        .ok_or("C12 Premium overview decode failed")?;
    evidence(&format!(
        "C12_PREMIUM state={:?} authority_revision={} remaining_us={}",
        overview.state, overview.authority_revision, overview.valid_for_us,
    ));
    if overview.state != premium_overview::State::Active || overview.valid_for_us == 0 {
        return Err("C12 durable Premium has no usable lease after fresh admission".into());
    }
    Ok(())
}

pub(super) struct ClientCaptureSetup {
    pub accounts: Vec<String>,
    pub characters: Vec<[u8; 16]>,
    pub definition: NonZeroU32,
    pub route: Vec<(StepDirection, TilePos)>,
    pub previous: oteryn_protocol_oteryn::GameSessionId,
    pub position: TilePos,
}

pub(super) const CLIENT_TARGET_NAME: &str = "Tarantula";
const CLIENT_TARGET_KEY: &str = "oteryn:creature.tarantula";
const CLIENT_TARGET_SPAWN: TilePos = TilePos {
    x: 32294,
    y: 32333,
    floor: -8,
};
const CLIENT_CAST_TILE: TilePos = TilePos {
    x: 32295,
    y: 32334,
    floor: -8,
};

fn client_cast_route(
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    from: TilePos,
) -> TestResult<Vec<(StepDirection, TilePos)>> {
    let authored = spawns.sources.iter().any(|source| {
        source.floor == CLIENT_TARGET_SPAWN.floor
            && source.points.iter().any(|point| {
                point.x == CLIENT_TARGET_SPAWN.x
                    && point.y == CLIENT_TARGET_SPAWN.y
                    && spawns
                        .creatures
                        .get(point.creature as usize)
                        .is_some_and(|creature| creature.key == CLIENT_TARGET_KEY)
            })
    });
    if !authored || from == CLIENT_CAST_TILE {
        return Err("C12.client authored hunt spawn or starting position differs".into());
    }
    let profiles = creature_profiles()?;
    if profiles
        .get(CLIENT_TARGET_KEY)
        .is_none_or(|profile| profile.health != 225 || profile.display_name != CLIENT_TARGET_NAME)
    {
        return Err("C12.client authored Tarantula profile differs".into());
    }
    let path = route(world, from, |at| at == CLIENT_CAST_TILE)
        .ok_or("C12.client no route to the authored Tarantula cast tile")?;
    let mut at = from;
    path.into_iter()
        .map(|direction| {
            let landed = destination(world, at, direction)
                .filter(|landed| *landed != at)
                .ok_or("C12.client planned movement has no changed landing")?;
            at = landed;
            Ok((direction, landed))
        })
        .collect::<Result<_, &'static str>>()
        .map_err(Into::into)
}

// Relog with the weapon equipped before approaching the hostile spawn.
pub(super) async fn prepare_client_capture(
    clients: &SeamClients<'_>,
) -> TestResult<ClientCaptureSetup> {
    character_prep::declare_seeded(
        "C12.client",
        "level-20-monk,premium,item-50271",
        "capture Greater Tiger Clash with the augment equipped",
    );
    let dedicated =
        character_prep::dedicated_character(clients, "NODE_BOOT_C12_MONK_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let dedicated_clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    if !requires_platform_grant(&dedicated_clients.accounts[1]) {
        return Err(super::full_row_c02::PLATFORM_ADMISSION_GAP.into());
    }
    let mut client = prepare_c12_monk(dedicated_clients).await?;
    character_prep::grant_premium(&client).await?;
    eprintln!("FIXTURE C12 Premium grant applied once for the dedicated Monk");
    character_prep::grant_item(&client, ITEM_ID, 1).await?;
    client = fresh_c12_admission(client, dedicated_clients).await?;
    verify_seeded_monk(&mut client).await?;
    let definition = item_ref()?;
    if carried_count(&client, definition) != 1 {
        return Err("C12.client expected one carried weapon before equip".into());
    }
    equip_right(&mut client, definition).await?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let previous = client.join_snapshot().game_session_id;
    let position = tile(client.world_spatial().actor_position)?;
    let route = client_cast_route(world, &metadata.spawns, position)?;
    evidence(&format!(
        "C12_CLIENT_HUNT spawn={CLIENT_TARGET_SPAWN:?} stand={CLIENT_CAST_TILE:?} steps={}",
        route.len()
    ));
    logout(&mut client).await?;
    Ok(ClientCaptureSetup {
        accounts,
        characters,
        definition,
        route,
        previous,
        position,
    })
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let definition = item_ref()?;
    let dedicated =
        character_prep::dedicated_character(clients, "NODE_BOOT_C12_MONK_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    if !requires_platform_grant(&clients.accounts[1]) {
        return Err(super::full_row_c02::PLATFORM_ADMISSION_GAP.into());
    }
    let mut client = prepare_c12_monk(clients).await?;
    verify_seeded_monk(&mut client).await?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    if carried_count(&client, definition) != 0 || equipped_right(&client, definition) {
        return Err(
            "item 50271 must first enter the dedicated Monk's backpack through the purchase".into(),
        );
    }
    character_prep::grant_premium(&client).await?;
    eprintln!("FIXTURE C12 Premium grant applied once for the dedicated Monk");
    character_prep::grant_item(&client, MANA_POTION, 12).await?;
    let coins = pema_planned_price()?.div_ceil(CRYSTAL_COIN_VALUE);
    character_prep::grant_item(&client, CRYSTAL_COIN, u32::try_from(coins)?).await?;
    client = fresh_c12_admission(client, clients).await?;
    verify_seeded_monk(&mut client).await?;
    let purchase_balance = acquire_pema(&mut client, world, &metadata.npcs, definition).await?;
    let acquired = carried_count(&client, definition);
    if acquired != 1 {
        return Err(format!(
            "expected one purchased item {ITEM_ID} in backpack; observed={acquired}"
        )
        .into());
    }
    row.pass("acquire-50271", (acquired, purchase_balance));

    require_current_premium(client.session_mut()).await?;
    let spell_index = spell(&mut client)?;
    let augment = content_augment()?;
    let profiles = creature_profiles()?;
    let hunt = select_hunt(&client, world, &metadata.spawns, &profiles)?;
    hunt.approach(&mut client).await?;

    let baseline_weapon = weapon(&client)?;
    if baseline_weapon == Some(definition) {
        return Err("need=augment-off-baseline;item-50271-already-equipped".into());
    }
    let baseline =
        verdict::indexed_timeout(BASELINE_BUDGET, samples(&hunt, &mut client, spell_index, 0))
            .await
            .map_err(|_| {
                format!(
                    "baseline sampling deadline=75s last-frame={}",
                    client.last_server_sequence()
                )
            })??;
    row.record_cast("baseline-cast", &baseline, baseline_weapon);

    equip_right(&mut client, definition).await?;
    let augmented = samples(&hunt, &mut client, spell_index, augment).await?;
    row.record_cast("augmented-cast", &augmented, weapon(&client)?);

    let equipped = client
        .inventory()
        .and_then(|i| {
            i.equipment.iter().find(|e| {
                e.slot == items::EquipmentSlot::RightHand
                    && e.item.item_definition_ref == definition
            })
        })
        .map(|e| e.item.handle)
        .ok_or("right-hand item absent before unequip")?;
    let moved = client
        .move_item(&items::ItemMoveIntent {
            source: equipped,
            destination: items::ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("unequip ITEM_MOVE reply={moved:?}").into());
    }
    wait_inventory(&mut client, "unequip", |c| {
        !equipped_right(c, definition) && carried_count(c, definition) == 1
    })
    .await?;
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
    if fresh.join_snapshot().game_session_id == old_session
        || !equipped_right(&fresh, definition)
        || carried_count(&fresh, definition) != 0
        || full_e2ef7::units(&fresh, definition) != 1
    {
        return Err("fresh admission lost the right-hand item".into());
    }
    let fresh_balance = full_e2ef7::player_gold(&mut fresh).await?;
    if fresh_balance != purchase_balance {
        return Err(format!(
            "Pema purchase balance changed after relog: expected {purchase_balance}, observed {fresh_balance}"
        ).into());
    }
    require_current_premium(fresh.session_mut()).await?;
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
        const MARKER: &str =
            "SEAM_EVIDENCE c12_base_damage session=test command=1 target=rat generation=2 ";
        let read = |tail: &str, bps| {
            let log = format!("{MARKER}{tail}");
            read_base_damage(log.as_bytes(), MARKER, bps)
        };
        assert_eq!(read("before=37 after=40", 1_000)?, (37, 40));
        assert_eq!(read("before=37 after=37", 0)?, (37, 37));
        assert!(read("before=37 after=41", 1_000).is_err());
        assert!(read("before=37 after=37", 1_000).is_err());
        assert!(read("before=37 after=40", 0).is_err());
        let duplicate = format!(
            "{MARKER}before=37 after=40
{MARKER}before=37 after=40"
        );
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
        let line = gap("baseline-cast", "CAST deadline", &STEPS[2..]);
        assert!(line.starts_with("C12_GAP step=baseline-cast observed=CAST deadline"));
        assert!(line.contains(
            "not-run=augmented-cast,unequip,unequipped-cast,fresh-admission,augment-attribution"
        ));
    }

    #[test]
    fn pass_requires_every_step_in_order() {
        let mut row = Row::new();
        row.pass("baseline-cast", "out of order");
        assert!(row.passed.is_empty());
        for step in STEPS {
            row.pass(step, "ok");
        }
        assert_eq!(row.passed.len(), STEPS.len());
    }

    #[test]
    fn content_binds_the_augment_to_the_item() -> TestResult {
        assert_eq!(content_augment()?, AUGMENT_BPS);
        assert_ne!(item_ref()?.get(), 0);
        Ok(())
    }

    #[test]
    fn the_declared_trade_sells_the_item() -> TestResult {
        assert_eq!(pema_planned_price()?, PEMA_PRICE);
        Ok(())
    }
}
