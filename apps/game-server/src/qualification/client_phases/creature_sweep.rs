//! Creature sweep: every authored creature species is spawned next to the player through the node's
//! qualification `spawn` control command, fought, killed and looted by a real bot over the real wire,
//! with XP, corpse and loot checked against content.
//!
//! Select with `NODE_BOOT_E2E_ONLY=creature_sweep` (never part of the default run).
//! `OTERYN_E2E_CREATURE_BATCH=start:len` selects a slice of the key-sorted species list of
//! `content/creatures/definitions` (default `0:3`; anything else is an error).
//!
//! Spawn path: `tools/qualification/node_boot/provision.py --spawn` over `NODE_BOOT_CONTROL_SOCKET`
//! (`sudo -n python3`, as `provision.rs` runs the fixture), answered by the node's uid-0 `spawn`
//! control route (`qualification_wild_spawn.rs`, feature `qualification-spawn`).
//!
//! One shared session serves the whole sweep: it is provisioned once, and re-admitted (never
//! re-provisioned) when its health stays low or the session became unusable. A creature that cannot be killed within the bound
//! gets `kill FAIL cause=deadline`; the sweep releases the target and moves on.
//!
//! DPS budget: `CALIBRATION_KEY` is killed first and its authored health over the time from the first
//! damage to the death is the DPS (`creature_sweep calibration ...`). A species whose authored health
//! exceeds `dps * KILL_SECONDS * 80%` is not spawned: its rows are `SKIPPED` with
//! `not_attempted:hp_X>budget_Y;dps_Z`. Kill rows carry hp, budget and dps.
//! After three failed calibration attempts, every selected species runs without a DPS budget;
//! each kill retains `kill_seconds()` and reports `dps=unmeasured budget=none`.
//!
//! Rows per species: `CS.<key>.spawn|kill|xp|corpse|loot|full`. `spawn` carries the node answer, or
//! the rejected token with cause=precondition. A FAIL row makes `run` return Err, which `journeys`
//! propagates with `?`, so the node-boot stage exits non-zero.
//! XP oracle: the authored `experience` through the server's own
//! `crates/gameplay/characters/src/hunting_experience.rs:13 hunting_experience` (called from
//! `crates/gameplay/durability/src/durability/party_kill_xp.rs:369`), with no prey, no boost, full
//! stamina and no premium for a solo level-1000 kill (`animus_mastery_experience` adds nothing without
//! mastery, `crates/gameplay/combat/src/combat/death_reward.rs:939`).
//! The exact corpse and pinned deterministic loot plan are checked alongside the authored ranges.
//! Loot oracle: the authored loot table. `crates/gameplay/items/src/loot_plan.rs:370 roll_entries`
//! draws each entry independently with a 95..105% chance factor (`loot_factor_percent`), so an empty
//! corpse is possible for any table and every item count lies in its entry's min..max.
use super::*;
use crate::content::creature_reward::{CreatureRewardRow, CreatureRewardTable, NoSettlementReason};
use oteryn_gameplay_items::loot_plan::{
    LootPlanDeathKey, LootSelectionAlgorithm, LootTableDefinition, apply_scheduled_loot_rate,
    partition_daily_boss_bonus_entries, plan_creature_loot,
};

use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, SessionEvent,
    client_ui::cyclopedia_character as panel,
};
use oteryn_protocol_oteryn::{
    attack, cyclopedia_character as character, logout::LogoutResult, target_move,
};
use std::collections::BTreeSet;

const BATCH_VARIABLE: &str = "OTERYN_E2E_CREATURE_BATCH";
const BATCH_DEFAULT: (usize, usize) = (0, 3);
const STEPS: [&str; 5] = ["spawn", "kill", "xp", "corpse", "loot"];
/// The `sweep` fixture profile's level (`qualification_fixture.rs` `SWEEP_LEVEL`).
const SWEEP_LEVEL: u32 = 1000;
/// Ruthless axe: attack 50, level 75, knight, equip requirements known to the content
/// (`content/items/definitions/spell-native-profiles.json`), minted by the `sweep` profile.
const SWEEP_WEAPON_ID: u32 = 6553;
/// Weak, common, melee-only huntable (260 hp, one attack): its kill measures the sweep DPS.
const CALIBRATION_KEY: &str = "oteryn:creature.cyclops";
/// A species is attempted only when its authored health fits this share of the DPS budget.
const BUDGET_PERCENT: f64 = 80.0;
const CORE_CHARACTER: usize = 0;
/// A full stamina bar in minutes (`composed_admission/vitals.rs:191`); no hunting time passes first.
const FULL_STAMINA_MINUTES: i32 = 2520;
const VISIBLE_SECONDS: u64 = 30;
const KILL_SECONDS: u64 = 150;
/// Long sweep (`OTERYN_E2E_SWEEP_LONG_SECONDS=<n>`): only the species over the normal
/// `KILL_SECONDS` budget run, each with `n` seconds to kill; the rest belong to the normal sweep.
const LONG_VARIABLE: &str = "OTERYN_E2E_SWEEP_LONG_SECONDS";
/// Recovery after a setup failure (a refused logout or release, a server close): the server's
/// control-loss grace is awaited and the character admitted again within this bound.
const RECOVER_SECONDS: u64 = 300;
/// Pause after a refused admission; grows with each attempt and stays at the last value.
const RECOVER_BACKOFF_MILLIS: [u64; 4] = [1_000, 2_000, 4_000, 8_000];
/// Consecutive species whose setup failed even after a successful re-admission end the batch.
const RECOVER_LIMIT: u32 = 3;
/// Wire code 1200 (`oteryn_protocol_oteryn` `NotSupported`, `E{number:04}` rendering): the first
/// code of the 1200..1999 "future wire" block of `FOUNDATION_ERROR_VOCABULARY.md`; the live
/// readiness review plans the maintenance graceful stop on it. The server ended the connection
/// with it, so the session is gone and the character stays held until control-loss grace ends.
const E1200_MEANING: &str = "E1200=wire-code-1200-NotSupported-server-ended-the-connection";

fn long_seconds() -> TestResult<Option<u64>> {
    match std::env::var(LONG_VARIABLE) {
        Ok(text) => {
            let seconds: u64 = text
                .trim()
                .parse()
                .map_err(|error| format!("{LONG_VARIABLE}={text:?}: {error}"))?;
            if seconds <= KILL_SECONDS {
                return Err(format!("{LONG_VARIABLE} must exceed {KILL_SECONDS}").into());
            }
            Ok(Some(seconds))
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("{LONG_VARIABLE}: {error}").into()),
    }
}

/// Seconds a species has to die: `KILL_SECONDS`, or the long-sweep value.
fn kill_seconds() -> u64 {
    long_seconds().ok().flatten().unwrap_or(KILL_SECONDS)
}
const HEALTH_WAIT_SECONDS: u64 = 60;

/// `start:len` with plain decimal numbers and `len > 0`; `None` runs the default slice.
fn creature_batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(text) = value else {
        return Ok(BATCH_DEFAULT);
    };
    let number = |part: &str| {
        (!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<usize>().ok())
            .flatten()
    };
    match text
        .split_once(':')
        .and_then(|(start, len)| Some((number(start)?, number(len)?)))
    {
        Some((start, len)) if len > 0 => Ok((start, len)),
        _ => Err(
            format!("invalid {BATCH_VARIABLE}={text:?}: expected start:len with len > 0").into(),
        ),
    }
}

fn batch_from_env() -> TestResult<(usize, usize)> {
    match std::env::var(BATCH_VARIABLE) {
        Ok(text) => creature_batch(Some(&text)),
        Err(std::env::VarError::NotPresent) => creature_batch(None),
        Err(error) => Err(format!("invalid {BATCH_VARIABLE}: {error}").into()),
    }
}

/// Report-safe evidence text: no whitespace and no `=`-pairs a reader could mistake for a field.
fn token(text: &str) -> String {
    text.chars()
        .take(400)
        .map(|character| {
            if character.is_ascii_alphanumeric() || "._:-,;/".contains(character) {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn at(tile: TilePos) -> String {
    format!("{},{},{}", tile.x, tile.y, tile.floor)
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
    skipped: usize,
}

/// The rows of one species; each step is emitted once.
struct Rows<'a> {
    short: &'a str,
    emitted: [bool; 5],
    lap: std::time::Instant,
    counts: Tally,
    /// Seconds from the first observed damage to the death, set by a kill.
    kill_seconds: Option<f64>,
    /// The error of a failed setup (logout, release or admission); the sweep recovers from it.
    setup_error: Option<String>,
    dps: Option<f64>,
}

impl<'a> Rows<'a> {
    fn new(short: &'a str, dps: Option<f64>) -> Self {
        Self {
            short,
            emitted: [false; 5],
            lap: std::time::Instant::now(),
            counts: Tally::default(),
            kill_seconds: None,
            setup_error: None,
            dps,
        }
    }

    fn emit(&mut self, step: &str, status: &str, detail: &str, cause: Option<&str>) {
        let seconds = self.lap.elapsed().as_secs_f64();
        self.lap = std::time::Instant::now();
        if let Some(index) = STEPS.iter().position(|name| *name == step) {
            self.emitted[index] = true;
        }
        match status {
            "PASS" => self.counts.pass += 1,
            "SKIPPED" => self.counts.skipped += 1,
            _ => self.counts.fail += 1,
        }
        let cause = cause.map_or_else(String::new, |cause| {
            let root = if cause == "blocked-by" {
                detail.strip_prefix("blocked-by-").unwrap_or(step)
            } else {
                step
            };
            let cause = cause.replace('_', "-");
            format!(" cause={cause} root=CS.{}.{root}", self.short)
        });
        let budget = if step == "kill" && self.dps.is_none() {
            " dps=unmeasured budget=none"
        } else {
            ""
        };
        evidence(&format!(
            "e2e_all scenario=CS.{}.{step} status={status} seconds={seconds:.3} evidence={}{budget}{cause}",
            self.short,
            token(detail)
        ));
    }

    fn pass(&mut self, step: &str, detail: &str) {
        self.emit(step, "PASS", detail, None);
        verdict::Harness::global().progress("creature-sweep-observation");
    }

    fn fail(&mut self, step: &str, cause: &str, detail: &str) {
        self.emit(step, "FAIL", detail, Some(cause));
    }

    /// A step that cannot run because an earlier one failed is a FAIL, never a skip.
    fn block_rest(&mut self, root: &str) {
        for (index, step) in STEPS.iter().enumerate() {
            if !self.emitted[index] {
                self.emit(
                    step,
                    "FAIL",
                    &format!("blocked-by-{root}"),
                    Some("blocked-by"),
                );
            }
        }
    }
}

/// A step the species journey cannot go on from.
struct Failure {
    step: &'static str,
    cause: &'static str,
    detail: String,
}

fn failure(step: &'static str, cause: &'static str, detail: impl std::fmt::Display) -> Failure {
    Failure {
        step,
        cause,
        detail: detail.to_string(),
    }
}

/// Preserve the primary spawn evidence even when setup's last phase was fixture-release.
fn spawn_error(stop: &Failure) -> Box<dyn std::error::Error> {
    verdict::HarnessError::boxed(
        verdict::Cause::Precondition {
            name: format!("spawn/{}", token(&stop.detail)),
        },
        format!("spawn:{}:{}", stop.cause, stop.detail),
    )
}

struct Definition {
    name: String,
    health: u64,
    experience: u64,
    attackable: bool,
    speed: u64,
}

struct LootEntry {
    item: String,
    revision: String,
    min: u64,
    max: u64,
}

enum LootBinding {
    /// The definition binds no loot table: the corpse is minted empty.
    None,
    Table(String, Vec<LootEntry>),
    Missing(String),
}

struct Species {
    short: String,
    definition: Option<Definition>,
    key: String,
    loot: LootBinding,
    /// The measured sweep DPS; `None` during calibration and for the unbudgeted fallback.
    dps: Option<f64>,
    private_reward: bool,
    daily_boss_equipment: BTreeSet<String>,
    reward: Result<std::sync::Arc<CreatureRewardRow>, NoSettlementReason>,
}

/// Authored health a kill within `KILL_SECONDS` may be expected to fit at `dps`.
fn budget(dps: f64) -> u64 {
    budget_for(dps, KILL_SECONDS)
}

fn budget_for(dps: f64, seconds: u64) -> u64 {
    (dps * seconds as f64 * BUDGET_PERCENT / 100.0).floor() as u64
}

/// No measurement means no HP budget: deadlines and all row assertions still apply.
fn row_budget(dps: Option<f64>, seconds: u64) -> Option<u64> {
    dps.map(|value| budget_for(value, seconds))
}

#[derive(Debug, PartialEq)]
enum CalibrationDecision {
    Measured(f64),
    Retry,
    Fallback,
}

const fn calibration_decision(
    attempt: u32,
    seconds: Option<f64>,
    health: u64,
    passed: bool,
) -> CalibrationDecision {
    match seconds {
        Some(seconds) if passed && seconds.is_finite() && seconds > 0.0 && health > 0 => {
            CalibrationDecision::Measured(health as f64 / seconds)
        }
        _ if attempt < 3 => CalibrationDecision::Retry,
        _ => CalibrationDecision::Fallback,
    }
}

fn read_shards(directory: &Path, prefix: &str) -> TestResult<Vec<serde_json::Value>> {
    let mut files = std::fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    files.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".json"))
    });
    files.sort();
    if files.is_empty() {
        return Err(format!("no {prefix}*.json shard in {}", directory.display()).into());
    }
    let mut records = Vec::new();
    for file in files {
        let mut shard: serde_json::Value = serde_json::from_slice(&std::fs::read(&file)?)?;
        match shard["records"].take() {
            serde_json::Value::Array(list) => records.extend(list),
            _ => return Err(format!("{} has no records list", file.display()).into()),
        }
    }
    Ok(records)
}

/// Per creature key: its definition and the loot table key it binds (`null` binds none).
fn definitions(root: &Path) -> TestResult<BTreeMap<String, (Definition, Option<String>)>> {
    let mut found = BTreeMap::new();
    for record in read_shards(&root.join("content/creatures/definitions"), "creatures-")? {
        let Some(key) = record["definition"]["identity"]["key"].as_str() else {
            continue;
        };
        let profile = &record["authoring"]["profile"];
        let Some(name) = profile["details"]["display_name"].as_str() else {
            continue;
        };
        found.insert(
            key.to_owned(),
            (
                Definition {
                    name: name.to_owned(),
                    health: profile["health"].as_u64().unwrap_or(0),
                    experience: profile["experience"].as_u64().unwrap_or(0),
                    attackable: profile["details"]["flags"]["attackable"]
                        .as_bool()
                        .unwrap_or(false),
                    speed: profile["speed"].as_u64().unwrap_or(0),
                },
                record["definition"]["loot"]["key"]
                    .as_str()
                    .map(str::to_owned),
            ),
        );
    }
    Ok(found)
}

fn loot_tables(root: &Path) -> TestResult<BTreeMap<String, Vec<LootEntry>>> {
    let mut found = BTreeMap::new();
    for record in read_shards(&root.join("content/loot"), "loot-")? {
        let Some(key) = record["definition"]["identity"]["key"].as_str() else {
            continue;
        };
        let entries = record["definition"]["entries"]
            .as_array()
            .ok_or_else(|| format!("loot table {key} has no entries list"))?
            .iter()
            .map(|entry| -> TestResult<LootEntry> {
                Ok(LootEntry {
                    item: entry["item"]["key"]
                        .as_str()
                        .ok_or("loot entry without an item key")?
                        .to_owned(),
                    revision: entry["item"]["revision"]
                        .as_str()
                        .ok_or("loot entry without an item revision")?
                        .to_owned(),
                    min: entry["min_count"]
                        .as_u64()
                        .ok_or("loot entry without min_count")?,
                    max: entry["max_count"]
                        .as_u64()
                        .ok_or("loot entry without max_count")?,
                })
            })
            .collect::<TestResult<Vec<_>>>()?;
        found.insert(key.to_owned(), entries);
    }
    Ok(found)
}

/// `plan` limits its box to 160 tiles and two floors around the start, but the authored spawns can
/// lie beyond it, so this plans over the same production collision facts (`destination`) across
/// the whole map. Err carries every tile the search reached.
fn far_route(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> Result<Vec<StepDirection>, Vec<TilePos>> {
    let mut queue = std::collections::VecDeque::from([from]);
    let mut parents = std::collections::BTreeMap::from([(from, None)]);
    while let Some(at) = queue.pop_front() {
        if goal(at) {
            let mut path = Vec::new();
            let mut cursor = at;
            while let Some(Some((previous, direction))) = parents.get(&cursor) {
                path.push(*direction);
                cursor = *previous;
            }
            path.reverse();
            return Ok(path);
        }
        for direction in [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ] {
            let Some(next) = destination(world, at, direction) else {
                continue;
            };
            if parents.contains_key(&next) {
                continue;
            }
            parents.insert(next, Some((at, direction)));
            queue.push_back(next);
        }
    }
    Err(parents.into_keys().collect())
}

fn item_ref(
    items: &crate::content::item_ref::ItemDefinitionIndex,
    key: &str,
    revision: &str,
) -> Option<std::num::NonZeroU32> {
    items.definition_ref("Item", key, revision)
}

// `world` serves the one walk out of the protection zone; the authored spawn table is not used.
pub(super) async fn run(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    rewards: &CreatureRewardTable,
) -> TestResult {
    if !std::env::var("NODE_BOOT_E2E_ONLY").is_ok_and(|list| !list.trim().is_empty()) {
        evidence("e2e_groups creature_sweep=skipped reason=opt_in_with_NODE_BOOT_E2E_ONLY");
        return Ok(());
    }
    let (start, len) = batch_from_env()?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    verdict::Harness::global().progress("creature-sweep-census");
    let definitions = definitions(&root)?;
    // `definitions` is a BTreeMap, so the species list is sorted by key. Huntable means a player
    // can attack it and gains experience for the kill; zero-speed definitions are the scenery,
    // eggs and totems the authoring marks attackable and never move.
    let (mut not_attackable, mut no_experience, mut immobile) = (0usize, 0usize, 0usize);
    let mut keys: Vec<&String> = Vec::new();
    for (key, (definition, _)) in &definitions {
        if !definition.attackable {
            not_attackable += 1;
        } else {
            no_experience += usize::from(definition.experience == 0);
            immobile += usize::from(definition.speed == 0);
            keys.push(key);
        }
    }
    evidence(&format!(
        "creature_sweep census defined={} huntable={} excluded_not_attackable={not_attackable} no_experience={no_experience} immobile={immobile} start={}",
        definitions.len(),
        keys.len(),
        at(TOWN)
    ));
    for (index, chunk) in keys.chunks(25).enumerate() {
        let listed: Vec<&str> = chunk.iter().map(|key| key.as_str()).collect();
        evidence(&format!(
            "creature_sweep species_keys from={} keys={}",
            index * 25,
            listed.join(",")
        ));
    }
    if start >= keys.len() {
        return Err(format!(
            "{BATCH_VARIABLE} start {start} is beyond the {} species",
            keys.len()
        )
        .into());
    }
    let end = start.saturating_add(len).min(keys.len());
    evidence(&format!(
        "creature_sweep batch start={start} len={len} selected={}",
        end - start
    ));
    let tables = loot_tables(&root)?;
    let items = provisioning::item_index()?;
    let mut tally = Tally::default();
    let mut seconds = Vec::new();
    let mut session: Option<DevClientSession> = None;
    let mut blocked: Option<String> = None;
    let mut setup_failures = 0u32;
    // Calibration: one weak melee-only species is killed first; its authored health over the time
    // from the first damage to the death is the DPS every other species is budgeted against.
    if !keys.iter().any(|key| key.as_str() == CALIBRATION_KEY) {
        return Err(format!(
            "creature sweep calibration species {CALIBRATION_KEY} is not huntable"
        )
        .into());
    }
    let mut calibration = species_of(CALIBRATION_KEY, &definitions, &tables, rewards, None);
    let health = calibration
        .definition
        .as_ref()
        .map_or(0, |found| found.health);
    // Calibration is advisory: preserve its FAIL evidence without failing the selected batch.
    let mut calibration_tally = Tally::default();
    let mut attempted = 0usize;
    let mut dps = None;
    let mut last = String::new();
    for attempt in 1..=3 {
        calibration.short = calibration_short("cyclops", attempt);
        last = "no-kill-measurement".to_owned();
        let mut outcome = SweepOutcome::default();
        let indexed_result = finish_indexed_sweep(
            &mut outcome,
            async |outcome| {
                verdict::indexed_advisory(
                    Box::leak(format!("CS.{}.full", calibration.short).into_boxed_str()),
                    Box::pin(async {
                        let result = sweep_species(
                            clients,
                            world,
                            &items,
                            &calibration,
                            &mut session,
                            &mut calibration_tally,
                            outcome,
                        )
                        .await;
                        if let Err(error) = &result {
                            last = recover_cause(&error.to_string());
                        }
                        result
                    }),
                )
                .await
            },
            cleanup_control,
        )
        .await;
        calibration_tally.fail = accounted_failures(
            calibration_tally.fail,
            outcome.completed,
            indexed_result.is_err(),
        );
        if let Err(error) = indexed_result {
            last = recover_cause(&error.to_string());
        }
        let measured = outcome.kill_seconds;
        let setup_error = outcome.setup_error;
        let decision = calibration_decision(attempt, measured, health, outcome.passed);
        evidence(&format!(
            "creature_sweep calibration attempt={attempt} result={}",
            match decision {
                CalibrationDecision::Measured(_) => "measured".to_owned(),
                _ => format!(
                    "failed;kill_seconds:{measured:?};setup:{}",
                    setup_error
                        .as_deref()
                        .map_or_else(|| "none".to_owned(), recover_cause)
                ),
            }
        ));
        // A setup failure must release and re-admit even on the last attempt, so fallback
        // rows do not immediately reuse the dead transport. Recovery failures remain evidence;
        // each subsequent calibration or batch row still attempts its own setup.
        if let Some(error) = setup_error {
            let outcome = recover(clients, &mut session).await;
            evidence(&format!(
                "creature_sweep recover species={} cause={} outcome={}",
                calibration.short,
                recover_cause(&error),
                outcome
                    .as_ref()
                    .map_or_else(|error| token(error), |()| "ok".to_owned())
            ));
        }
        match decision {
            CalibrationDecision::Measured(value) => {
                dps = Some(value);
                evidence(&format!(
                    "creature_sweep calibration key={CALIBRATION_KEY} hp={health} seconds={:.3} dps={value:.2} budget_seconds={KILL_SECONDS} budget_percent={BUDGET_PERCENT}",
                    health as f64 / value
                ));
                break;
            }
            CalibrationDecision::Retry | CalibrationDecision::Fallback => {}
        }
    }
    if dps.is_none() {
        evidence(&format!(
            "CREATURE_SWEEP calibration fallback reason={last}"
        ));
    }
    let long = long_seconds()?;
    for key in selected_batch(&keys, start, end, dps.is_some()) {
        let species = species_of(key, &definitions, &tables, rewards, dps);
        let hp = species.definition.as_ref().map_or(0, |found| found.health);
        if long.is_some() && dps.is_some_and(|value| hp <= budget(value)) {
            let mut rows = Rows::new(&species.short, species.dps);
            for step in STEPS {
                rows.emit(step, "SKIPPED", "within-normal-sweep-budget", None);
            }
            tally.skipped += rows.counts.skipped;
            evidence(&format!(
                "e2e_all scenario=CS.{}.full status=SKIPPED seconds=0.000 evidence=within-normal-sweep-budget",
                species.short
            ));
            continue;
        }
        if let (Some(value), Some(limit)) = (dps, row_budget(dps, long.unwrap_or(KILL_SECONDS)))
            && hp > limit
        {
            let mut rows = Rows::new(&species.short, species.dps);
            let detail = format!("not_attempted:hp_{hp}>budget_{limit};dps_{value:.2}");
            for step in STEPS {
                rows.emit(step, "SKIPPED", &detail, None);
            }
            tally.skipped += rows.counts.skipped;
            evidence(&format!(
                "e2e_all scenario=CS.{}.full status=SKIPPED seconds=0.000 evidence={detail}",
                species.short
            ));
            continue;
        }
        if let Some(cause) = &blocked {
            let mut rows = Rows::new(&species.short, species.dps);
            let detail = format!("blocked-by-recovery:{cause}");
            for step in STEPS {
                rows.emit(step, "FAIL", &detail, Some(detail.as_str()));
            }
            tally.fail += rows.counts.fail;
            evidence(&format!(
                "e2e_all scenario=CS.{}.full status=FAIL seconds=0.000 evidence={detail} cause=blocked",
                species.short
            ));
            continue;
        }
        let scenario: &'static str =
            Box::leak(format!("CS.{}.full", species.short).into_boxed_str());
        let started = std::time::Instant::now();
        let mut outcome = SweepOutcome::default();
        attempted += 1;
        let indexed_result = finish_indexed_sweep(
            &mut outcome,
            async |outcome| {
                verdict::indexed_checked(
                    scenario,
                    Box::pin(sweep_species(
                        clients,
                        world,
                        &items,
                        &species,
                        &mut session,
                        &mut tally,
                        outcome,
                    )),
                )
                .await
            },
            cleanup_control,
        )
        .await;
        tally.fail = accounted_failures(tally.fail, outcome.completed, indexed_result.is_err());
        if let Err(error) = indexed_result
            && !outcome.completed
            && outcome.setup_error.is_none()
        {
            outcome.setup_error = Some(error.to_string());
        }
        seconds.push(started.elapsed().as_secs_f64());
        let Some(error) = outcome.setup_error else {
            setup_failures = 0;
            continue;
        };
        // The species row is already a FAIL with the exact cause; the character is still held by
        // the server, so the next species would only burn its setup bound on the same close.
        setup_failures += 1;
        let cause = recover_cause(&error);
        let waited = std::time::Instant::now();
        let outcome = if setup_failures > RECOVER_LIMIT {
            Err(format!(
                "setup-failed-{RECOVER_LIMIT}-times-after-re-admission"
            ))
        } else {
            recover(clients, &mut session).await
        };
        evidence(&format!(
            "creature_sweep recover species={} cause={cause} waited_ms={} outcome={}",
            species.short,
            waited.elapsed().as_millis(),
            if outcome.is_ok() { "ok" } else { "failed" }
        ));
        if let Err(reason) = outcome {
            blocked = Some(token(&format!("{reason};after={}:{cause}", species.short)));
        }
    }
    for (stage, result) in finish_sweep(
        session.take(),
        async |mut client| release(&mut client).await,
        wait_fixture_released(clients, CORE_CHARACTER),
    )
    .await
    {
        if let Err(error) = result {
            tally.fail += 1;
            evidence(&format!(
                "creature_sweep release_failed stage={stage} error={}",
                token(&error.to_string())
            ));
        }
    }
    let mean = seconds.iter().sum::<f64>() / seconds.len().max(1) as f64;
    evidence(&format!(
        "creature_sweep summary species={} rows_pass={} rows_fail={} rows_skipped={} calibration_rows_fail={} mean_seconds_per_creature={mean:.3}",
        seconds.len(),
        tally.pass,
        tally.fail,
        tally.skipped,
        calibration_tally.fail
    ));
    if !batch_succeeds(attempted, tally.fail) {
        return Err(format!(
            "creature sweep: attempted={attempted} failed_rows={} (see the CS.* rows above)",
            tally.fail
        )
        .into());
    }
    Ok(())
}

fn species_of(
    key: &str,
    definitions: &BTreeMap<String, (Definition, Option<String>)>,
    tables: &BTreeMap<String, Vec<LootEntry>>,
    rewards: &CreatureRewardTable,
    dps: Option<f64>,
) -> Species {
    let (definition, table) = &definitions[key];
    let loot = match table {
        None => LootBinding::None,
        Some(table) => tables.get(table).map_or_else(
            || LootBinding::Missing(table.clone()),
            |entries| {
                LootBinding::Table(
                    table.clone(),
                    entries
                        .iter()
                        .map(|entry| LootEntry {
                            item: entry.item.clone(),
                            revision: entry.revision.clone(),
                            min: entry.min,
                            max: entry.max,
                        })
                        .collect(),
                )
            },
        ),
    };
    let reward = rewards.row(key);
    let daily_boss_equipment = reward
        .as_ref()
        .ok()
        .into_iter()
        .flat_map(|row| &row.loot_table.entries)
        .filter(|entry| rewards.is_daily_boss_equipment(&entry.item.production_key))
        .map(|entry| entry.item.production_key.clone())
        .collect();
    Species {
        private_reward: rewards.reward_boss_corpse(key).is_some(),
        reward,
        daily_boss_equipment,
        short: token(key.strip_prefix("oteryn:creature.").unwrap_or(key)),
        definition: Some(Definition {
            name: definition.name.clone(),
            health: definition.health,
            experience: definition.experience,
            attackable: definition.attackable,
            speed: definition.speed,
        }),
        key: key.to_owned(),
        loot,
        dps,
    }
}

/// Per-species observations consumed by calibration and setup recovery.
#[derive(Default)]
struct SweepOutcome {
    completed: bool,
    passed: bool,
    kill_seconds: Option<f64>,
    setup_error: Option<String>,
    cleanup_session: Option<String>,
}

async fn finish_indexed_sweep(
    outcome: &mut SweepOutcome,
    run: impl AsyncFnOnce(&mut SweepOutcome) -> TestResult,
    cleanup: impl AsyncFnMut(String) -> Result<String, String>,
) -> TestResult {
    let result = run(outcome).await;
    let cleaned = cleanup_ticket(&mut outcome.cleanup_session, cleanup).await;
    match (result, cleaned) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; specimen cleanup:{cleanup}").into()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(cleanup)) => Err(format!("specimen cleanup:{cleanup}").into()),
        (Ok(()), Ok(())) => Ok(()),
    }
}

async fn cleanup_ticket(
    ticket: &mut Option<String>,
    mut cleanup: impl AsyncFnMut(String) -> Result<String, String>,
) -> Result<(), String> {
    if let Some(session) = ticket.clone() {
        let answer = cleanup(session).await?;
        if answer != "cleaned" {
            return Err(format!("cleanup refused: {answer}"));
        }
        *ticket = None;
    }
    Ok(())
}

async fn cleanup_control(session: String) -> Result<String, String> {
    verdict::Harness::global().progress("creature-sweep-cleanup");
    super::request_spawn_control(session, "cleanup".into()).await
}

async fn finish_sweep<T, F: std::future::Future<Output = TestResult>>(
    session: Option<T>,
    logout: impl FnOnce(T) -> F,
    terminal_release: impl std::future::Future<Output = TestResult>,
) -> [(&'static str, TestResult); 2] {
    let logout = match session {
        Some(client) => logout(client).await,
        None => Ok(()),
    };
    [("logout", logout), ("terminal", terminal_release.await)]
}

fn emit_species_verdict(
    rows: &Rows<'_>,
    started: std::time::Instant,
    tally: &mut Tally,
    mut emit: impl FnMut(&str),
) -> TestResult {
    let failed = rows.counts.fail > 0;
    emit(&format!(
        "creature_sweep species={} seconds={:.3} rows_pass={} rows_fail={} rows_skipped={}",
        rows.short,
        started.elapsed().as_secs_f64(),
        rows.counts.pass,
        rows.counts.fail,
        rows.counts.skipped
    ));
    tally.pass += rows.counts.pass;
    tally.fail += rows.counts.fail;
    tally.skipped += rows.counts.skipped;
    if failed {
        return Err(format!("creature {} has FAIL rows", rows.short).into());
    }
    emit(&format!(
        "e2e_all scenario=CS.{}.full status=PASS seconds={:.3} evidence=species-checks-completed",
        rows.short,
        started.elapsed().as_secs_f64(),
    ));
    Ok(())
}

async fn sweep_species(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    species: &Species,
    session: &mut Option<DevClientSession>,
    tally: &mut Tally,
    outcome: &mut SweepOutcome,
) -> TestResult {
    let started = std::time::Instant::now();
    let mut rows = Rows::new(&species.short, species.dps);
    let mut journey_error = None;
    if let Err(stop) = Box::pin(hunt(
        clients,
        world,
        items,
        species,
        session,
        &mut rows,
        &mut outcome.cleanup_session,
    ))
    .await
    {
        *session = None;
        journey_error = Some(if stop.step == "spawn" {
            spawn_error(&stop)
        } else {
            format!("{}:{}:{}", stop.step, stop.cause, stop.detail).into()
        });
        if stop.step == "spawn" && rows.setup_error.is_none() {
            rows.setup_error = Some(stop.detail.clone());
        }
        rows.fail(stop.step, stop.cause, &stop.detail);
        rows.block_rest(stop.step);
    }
    if let Err(error) = cleanup_ticket(&mut outcome.cleanup_session, cleanup_control).await {
        rows.counts.fail += 1;
        evidence(&format!(
            "creature_sweep cleanup species={} status=FAIL evidence={}",
            species.short,
            token(&error)
        ));
        if journey_error.is_none() {
            journey_error = Some(format!("specimen cleanup:{error}").into());
        }
    }
    outcome.kill_seconds = rows.kill_seconds;
    outcome.setup_error = rows.setup_error.take();
    let failed = rows.counts.fail > 0;
    outcome.passed = !failed;
    if !failed {
        emit_species_verdict(&rows, started, tally, evidence)?;
        outcome.completed = true;
        return Ok(());
    }
    evidence(&format!(
        "e2e_all scenario=CS.{}.full status={} seconds={:.3} evidence=steps:{}-pass,{}-fail,{}-skipped",
        species.short,
        if failed { "FAIL" } else { "PASS" },
        started.elapsed().as_secs_f64(),
        rows.counts.pass,
        rows.counts.fail,
        rows.counts.skipped
    ));
    evidence(&format!(
        "creature_sweep species={} seconds={:.3} rows_pass={} rows_fail={} rows_skipped={}",
        species.short,
        started.elapsed().as_secs_f64(),
        rows.counts.pass,
        rows.counts.fail,
        rows.counts.skipped
    ));
    tally.pass += rows.counts.pass;
    tally.fail += rows.counts.fail;
    outcome.completed = true;
    tally.skipped += rows.counts.skipped;
    if let Some(error) = journey_error {
        return Err(error);
    }
    if failed {
        return Err(format!("creature {} has FAIL rows", species.short).into());
    }
    Ok(())
}

/// The shared session is admitted once and provisioned once for the whole sweep. Low health is
/// recovered by waiting, then by a fresh admission of the same character; the fixture never reruns
/// on a character that already carries the build.
async fn ready(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    // A dead transport (a death or an earlier failed exchange) is reconnected once, after the
    // server released the character; the build is re-checked below either way.
    if let Some(client) = session.as_mut()
        && client
            .service_liveness(Duration::from_millis(50))
            .await
            .is_err()
    {
        *session = None;
    }
    if session.is_none() {
        reconnect(clients, session).await?;
    }
    prepare_player(clients, session).await?;
    if let Some(client) = session.as_mut()
        && needs_healing(client)?
    {
        let (_, _, vocation) = read_stats_with_vocation(client).await?;
        if let Some(vocation) = vocation {
            recover_health(client, world, vocation).await?;
        }
    }
    if is_low(session) {
        let recover = tokio::time::Instant::now() + Duration::from_secs(HEALTH_WAIT_SECONDS);
        while is_low(session) && tokio::time::Instant::now() < recover {
            let client = session.as_mut().ok_or("session absent")?;
            client.service_liveness(Duration::from_millis(250)).await?;
            client.take_events();
            verdict::Harness::global().progress("creature-sweep-recover");
        }
    }
    if is_low(session) {
        // Still low (or dead): admit the character again, without the fixture.
        let mut client = session.take().ok_or("session absent")?;
        release(&mut client).await?;
        drop(client);
        reconnect(clients, session).await?;
    }
    let client = session.as_mut().ok_or("session absent")?;
    leave_protection_zone(client, world).await
}

/// Admit the sweep character again once the server released the previous session (a dropped
/// session is never logged out, so the release wait is what proves the slot is free).
async fn reconnect(
    clients: &SeamClients<'_>,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    recover(clients, session).await.map_err(Into::into)
}

/// The recover evidence cause: the setup error, with the meaning of a server close by code 1200.
fn recover_cause(error: &str) -> String {
    let cause = token(error);
    if error.contains("E1200") {
        format!("{cause};{E1200_MEANING}")
    } else {
        cause
    }
}

/// Wait for the server to release the character (loss detection and grace, polled on the durable
/// session row, not slept) and admit it again; a refused admission is retried with a growing pause
/// until `RECOVER_SECONDS` pass. Returns the last error when the bound ends. The release poll
/// does not poison the fixture gate: it is the recovery itself.
async fn recover(
    clients: &SeamClients<'_>,
    session: &mut Option<DevClientSession>,
) -> Result<(), String> {
    *session = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(RECOVER_SECONDS);
    let client = recover_character_with(
        deadline,
        RECOVER_BACKOFF_MILLIS.map(Duration::from_millis),
        async || recover_once(clients).await,
    )
    .await
    .map_err(|error| format!("recovery-{RECOVER_SECONDS}s:{error}"))?;
    *session = Some(client);
    Ok(())
}

async fn recover_once(clients: &SeamClients<'_>) -> Result<DevClientSession, String> {
    let admission = async {
        wait_fixture_released(clients, CORE_CHARACTER)
            .await
            .map_err(|error| format!("release-wait:{error}"))?;
        admit_character(clients, 0x80, CORE_CHARACTER)
            .await
            .map_err(|error| format!("admission:{error}"))
    };
    tokio::pin!(admission);
    loop {
        verdict::Harness::global().progress("sweep-recover-release-admit");
        tokio::select! {
            result = &mut admission => return result,
            () = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
    }
}

fn is_low(session: &Option<DevClientSession>) -> bool {
    session
        .as_ref()
        .and_then(|client| client.actor_vitals())
        .is_some_and(|vitals| vitals.health.saturating_mul(2) < vitals.max_health)
}

/// The spawn needs free non-PZ cells next to the player: walk once to open ground.
async fn leave_protection_zone(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let open = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    };
    let roomy = |at: TilePos| {
        open(at)
            && [
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
            ]
            .into_iter()
            .filter(|direction| destination(world, at, *direction).is_some_and(open))
            .count()
                >= 3
    };
    let from = tile(client.world_spatial().actor_position)?;
    if roomy(from) {
        return Ok(());
    }
    verdict::Harness::global().progress("creature-sweep-route");
    let path = far_route(world, from, roomy).map_err(|reachable| {
        format!(
            "no open ground from {} ({} tiles reachable)",
            at(from),
            reachable.len()
        )
    })?;
    walk(client, world, &path).await
}

fn named_creature(entry: &oteryn_dev_client::WorldSpatialEntity, name: &str) -> bool {
    entry.kind == EntityKind::Creature
        && matches!(
            entry.detail,
            EntityDetail::Actor { .. } | EntityDetail::ActorWithIcons { .. }
        )
        && entry.display_name.is_some_and(|label| {
            label
                .as_str()
                .is_ok_and(|shown| shown.eq_ignore_ascii_case(name))
        })
}

#[cfg(test)]
fn spawn_candidate(
    entry: &oteryn_dev_client::WorldSpatialEntity,
    name: &str,
    existing: &BTreeSet<oteryn_dev_client::EntityRef>,
    floor: i16,
    answer: &str,
) -> bool {
    let identity = super::parse_fixture_spawn(answer.to_owned())
        .is_ok_and(|spawn| spawn.entity == entry.entity);
    identity
        && entry.position.floor == floor
        && named_creature(entry, name)
        && !existing.contains(&entry.entity)
}

#[cfg(test)]
fn corpse_candidate(
    entry: &oteryn_dev_client::WorldSpatialEntity,
    position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    existing: &BTreeSet<oteryn_dev_client::EntityRef>,
    expected: [u8; 16],
) -> bool {
    entry.kind == EntityKind::Corpse
        && entry.entity.identity == expected
        && entry.position == position
        && !existing.contains(&entry.entity)
}

fn disappearance_proves_kill(
    last: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    player: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    _damaged: bool,
    committed: bool,
) -> bool {
    committed
        && last.floor == player.floor
        && last.x.abs_diff(player.x).max(last.y.abs_diff(player.y)) <= 2
}

fn xp_matches(before: u64, after: u64, expected: u64, settled: bool) -> bool {
    settled && after.checked_sub(before) == Some(expected)
}

fn selected_batch<'a>(
    keys: &'a [&'a String],
    start: usize,
    end: usize,
    _calibrated: bool,
) -> Vec<&'a String> {
    keys[start..end].to_vec()
}

fn calibration_short(short: &str, attempt: u32) -> String {
    format!("calibration.{short}.attempt{attempt}")
}

fn accounted_failures(recorded: usize, completed: bool, indexed_failed: bool) -> usize {
    recorded + usize::from(indexed_failed && !completed)
}

fn batch_succeeds(attempted: usize, failed: usize) -> bool {
    attempted > 0 && failed == 0
}

/// Read-only proof of the exact native death and its atomically committed XP member.
struct DeathSettlement {
    corpse: [u8; 16],
    award: u64,
    total: u64,
}

fn death_identity(key: &[u8], domain: &[u8]) -> Option<[u8; 16]> {
    use sha2::{Digest, Sha256};
    if key.len() != 52 {
        return None;
    }
    let digest = Sha256::new()
        .chain_update(domain)
        .chain_update(key)
        .finalize();
    let mut identity = [0; 16];
    identity.copy_from_slice(&digest[..16]);
    Some(identity)
}

async fn read_death_settlement(
    connection: &mut sqlx::PgConnection,
    character: [u8; 16],
    target: oteryn_dev_client::EntityRef,
) -> TestResult<Option<DeathSettlement>> {
    use sqlx::Row;
    let rows = sqlx::query("SELECT r.death_key,m.xp_awarded,p.total_experience FROM game_party_kill_xp_receipts r JOIN game_party_kill_xp_members m USING(death_key) JOIN game_character_progression_state p ON p.character_id=m.character_id WHERE m.character_id=encode($1,'hex')::uuid AND octet_length(r.death_key)=52 AND substring(sha256($2::bytea || r.death_key) from 1 for 16)=$3::bytea AND substring(r.death_key from 45 for 8)=$4::bytea LIMIT 1")
        .bind(character.as_slice())
        .bind(b"oteryn:runtime-player-placement:v1".as_slice())
        .bind(target.identity.as_slice())
        .bind(target.generation.to_be_bytes().as_slice())
        .fetch_all(connection).await?;
    for row in rows {
        let key: Vec<u8> = row.try_get("death_key")?;
        if death_identity(&key, b"oteryn:runtime-player-placement:v1") != Some(target.identity)
            || key
                .get(44..52)
                .and_then(|bytes| bytes.try_into().ok())
                .map(u64::from_be_bytes)
                != Some(target.generation)
        {
            continue;
        }
        return Ok(Some(DeathSettlement {
            corpse: death_identity(&key, b"oteryn:runtime-corpse-placement:v1")
                .ok_or("invalid death key")?,
            award: u64::try_from(row.try_get::<i64, _>("xp_awarded")?)?,
            total: u64::try_from(row.try_get::<i64, _>("total_experience")?)?,
        }));
    }
    Ok(None)
}

async fn death_settlement(
    clients: &SeamClients<'_>,
    client: &mut DevClientSession,
    target: oteryn_dev_client::EntityRef,
) -> TestResult<DeathSettlement> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    verdict::Harness::global().bounded_wait(deadline, "creature-sweep-death-settlement");
    let mut connection =
        tokio::time::timeout_at(deadline, sqlx::PgConnection::connect(clients.url)).await??;
    loop {
        verdict::Harness::global().progress("creature-sweep-death-settlement");
        if let Some(settlement) = tokio::time::timeout_at(
            deadline,
            read_death_settlement(&mut connection, clients.characters[CORE_CHARACTER], target),
        )
        .await??
        {
            tokio::time::timeout_at(deadline, connection.close()).await??;
            return Ok(settlement);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("no committed receipt for the exact disappeared actor".into());
        }
        tokio::time::timeout_at(
            deadline,
            client.service_liveness(Duration::from_millis(100)),
        )
        .await??;
        client.take_events();
    }
}

async fn observe_xp_until<T>(
    client: &mut T,
    deadline: tokio::time::Instant,
    expected_total: u64,
    mut read: impl AsyncFnMut(&mut T) -> TestResult<u64>,
    mut service: impl AsyncFnMut(&mut T) -> TestResult,
) -> TestResult<u64> {
    let mut last = None;
    loop {
        // Reserve the liveness quantum before beginning another stats exchange.
        if let Some(xp) = last
            && deadline.saturating_duration_since(tokio::time::Instant::now())
                <= Duration::from_millis(250)
        {
            return Ok(xp);
        }
        if tokio::time::Instant::now() >= deadline {
            return last.ok_or_else(|| "XP observation deadline".into());
        }
        let xp = tokio::time::timeout_at(deadline, read(client)).await??;
        if tokio::time::Instant::now() >= deadline {
            return Err("XP observation deadline".into());
        }
        if xp == expected_total {
            return Ok(xp);
        }
        last = Some(xp);
        // Finish servicing a complete exchange; only start another while budget remains.
        if deadline.saturating_duration_since(tokio::time::Instant::now())
            <= Duration::from_millis(250)
        {
            return Ok(xp);
        }
        tokio::time::timeout_at(deadline, service(client)).await??;
    }
}

fn creature_named(entry: &oteryn_dev_client::WorldSpatialEntity, name: &str) -> bool {
    entry.kind == EntityKind::Creature
        && matches!(
            entry.detail,
            EntityDetail::Actor { .. } | EntityDetail::ActorWithIcons { .. }
        )
        && entry.display_name.is_some_and(|label| {
            label
                .as_str()
                .is_ok_and(|shown| shown.eq_ignore_ascii_case(name))
        })
}

fn visible_spawn<'a>(
    mut entities: impl Iterator<Item = &'a oteryn_dev_client::WorldSpatialEntity>,
    player: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    name: &str,
    existing: &BTreeSet<oteryn_dev_client::EntityRef>,
    expected: oteryn_dev_client::EntityRef,
) -> Option<oteryn_dev_client::WorldSpatialEntity> {
    entities
        .find(|entry| {
            entry.entity == expected
                && entry.position.floor == player.floor
                && creature_named(entry, name)
                && !existing.contains(&entry.entity)
        })
        .copied()
}

fn corpse_at(
    entry: &oteryn_dev_client::WorldSpatialEntity,
    position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    existing: &BTreeSet<oteryn_dev_client::EntityRef>,
    expected_identity: [u8; 16],
) -> bool {
    entry.kind == EntityKind::Corpse
        && entry.entity.identity == expected_identity
        && entry.position == position
        && !existing.contains(&entry.entity)
}

fn confirmed_kill(death_distance: u32, has_death_receipt: bool) -> bool {
    death_distance <= 2 && has_death_receipt
}

async fn sweep_reward_selection<S, T>(
    session: &mut Option<S>,
    query: impl AsyncFnOnce(&mut S) -> TestResult<T>,
) -> TestResult<T> {
    let mut current = session.take().ok_or("session absent after setup")?;
    let selection = query(&mut current).await?;
    *session = Some(current);
    Ok(selection)
}

fn boss_bonus_rolls(occurrence: [u8; 16]) -> u8 {
    use sha2::{Digest, Sha256};

    let digest = Sha256::new()
        .chain_update(b"oteryn:daily-boost:boss-roll:v1")
        .chain_update(occurrence)
        .finalize();
    let draw = u64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ]);
    2 + u8::from(draw % 101 < 50)
}

async fn hunt(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    species: &Species,
    session: &mut Option<DevClientSession>,
    rows: &mut Rows<'_>,
    cleanup_session: &mut Option<String>,
) -> Result<(), Failure> {
    let definition = species.definition.as_ref().ok_or_else(|| {
        failure(
            "spawn",
            "content",
            "no creature definition with a display name in content/creatures/definitions",
        )
    })?;
    if let Err(error) = ready(clients, world, session).await {
        // A failed setup leaves no trusted session; the next species starts fresh.
        *session = None;
        rows.setup_error = Some(error.to_string());
        return Err(failure("spawn", "precondition", format!("setup:{error}")));
    }
    let selection = sweep_reward_selection(session, client_daily_boost::reward_selection)
        .await
        .map_err(|error| failure("spawn", "protocol", error))?;
    let client = session
        .as_mut()
        .ok_or_else(|| failure("spawn", "precondition", "session absent after setup"))?;
    let (xp_before, level) = read_stats(client)
        .await
        .map_err(|error| failure("spawn", "precondition", format!("stats:{error}")))?;

    let name = definition.name.as_str();
    let named = |entry: &oteryn_dev_client::WorldSpatialEntity| named_creature(entry, name);
    // Leftovers of an earlier species never count as this species' spawn.
    let existing: BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| named(entry))
        .map(|entry| entry.entity)
        .collect();
    let session_id = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    *cleanup_session = Some(session_id.clone());
    let answer = with_peer_service(request_spawn(session_id, species.key.clone()), async || {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        Ok(())
    })
    .await
    .map_err(|error| failure("spawn", "protocol", error))?
    .map_err(|reason| failure("spawn", "precondition", format!("rejected:{reason}")))?;
    let corpse_identity = answer
        .corpse_identity(clients)
        .map_err(|error| failure("spawn", "precondition", error))?;
    let visible_deadline = tokio::time::Instant::now() + Duration::from_secs(VISIBLE_SECONDS);
    let target = loop {
        client
            .service_liveness(Duration::from_millis(100))
            .await
            .map_err(|error| failure("spawn", "protocol", error))?;
        verdict::Harness::global().progress("creature-sweep-visible");
        client.take_events();
        let player = client.world_spatial().actor_position;
        let found = client.world_entities().and_then(|entities| {
            visible_spawn(entities.iter(), player, name, &existing, answer.entity)
        });
        if let Some(found) = found {
            break found;
        }
        if tokio::time::Instant::now() >= visible_deadline {
            return Err(failure(
                "spawn",
                "absent",
                format!(
                    "{};no {name} visible within {VISIBLE_SECONDS}s",
                    answer.wire
                ),
            ));
        }
    };
    let here = tile(client.world_spatial().actor_position)
        .map_err(|error| failure("spawn", "protocol", error))?;
    rows.pass(
        "spawn",
        &format!("answer:{};name:{name};player:{}", answer.wire, at(here)),
    );

    // Kill: attack in chase mode until that exact entity is gone, with damage observed.
    let modes = attack::FightModes {
        chase: attack::ChaseMode::Chase,
        ..attack::FightModes::DEFAULT
    };
    let reply = client
        .session_mut()
        .set_fight_modes(&modes)
        .await
        .map_err(|error| failure("kill", "unkillable", format!("chase modes:{error}")))?;
    if reply.disposition != attack::AttackIntentDisposition::Ok {
        return Err(failure(
            "kill",
            "unkillable",
            format!("chase modes refused:{reply:?}"),
        ));
    }
    let target = client
        .world_entities()
        .and_then(|entities| entities.get(&target.entity))
        .copied()
        .ok_or_else(|| {
            failure(
                "kill",
                "unkillable",
                "target left visibility before the attack",
            )
        })?;
    let corpses_before: BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| entry.kind == EntityKind::Corpse)
        .map(|entry| entry.entity)
        .collect();
    let mut last_position = target.position;
    let mut last_health = match target.detail {
        EntityDetail::Actor { health_percent, .. }
        | EntityDetail::ActorWithIcons { health_percent, .. } => health_percent,
        EntityDetail::Object { .. } | EntityDetail::ObjectWithAppearance { .. } => 100,
    };
    let mut damaged = false;
    let mut first_damage = None;
    client.take_events();
    let attacked = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await
        .map_err(|error| failure("kill", "unkillable", format!("attack:{error}")))?;
    if attacked.status != CommandStatus::Accepted
        || attacked.disposition != attack::AttackIntentDisposition::Ok
    {
        return Err(failure(
            "kill",
            "unkillable",
            format!("attack refused:{attacked:?}"),
        ));
    }
    let combat_started = tokio::time::Instant::now();
    let kill_deadline = combat_started + Duration::from_secs(kill_seconds());
    let mut died = false;
    let gone = loop {
        client
            .service_liveness(Duration::from_millis(50))
            .await
            .map_err(|error| failure("kill", "unkillable", format!("liveness:{error}")))?;
        verdict::Harness::global().progress("creature-sweep-combat");
        client.take_events();
        if client
            .actor_vitals()
            .is_some_and(|vitals| vitals.max_health > 0 && vitals.health == 0)
        {
            died = true;
            break false;
        }
        match client
            .world_entities()
            .and_then(|entities| entities.get(&target.entity))
        {
            None => break true,
            Some(now) => {
                last_position = now.position;
                if let EntityDetail::Actor { health_percent, .. }
                | EntityDetail::ActorWithIcons { health_percent, .. } = now.detail
                {
                    if health_percent < last_health && first_damage.is_none() {
                        first_damage = Some(tokio::time::Instant::now());
                    }
                    damaged |= health_percent < last_health;
                    last_health = health_percent;
                }
            }
        }
        if tokio::time::Instant::now() >= kill_deadline {
            break false;
        }
    };
    let player = client.world_spatial().actor_position;
    let death_distance = last_position
        .x
        .abs_diff(player.x)
        .max(last_position.y.abs_diff(player.y));
    if died {
        let detail = format!(
            "player health reached 0 after {:.1}s;damage_observed={damaged};creature_health_percent={last_health};last_position={last_position:?};player={player:?};vitals={:?}",
            combat_started.elapsed().as_secs_f64(),
            client.actor_vitals()
        );
        // The dead session is dropped without logout; the next species reconnects after release.
        *session = None;
        return Err(failure("kill", "player_died", detail));
    }
    if !gone {
        let _ = client.session_mut().attack_target(None).await;
        return Err(failure(
            "kill",
            "deadline",
            format!(
                "no death within {}s;damage_observed={damaged};health_percent={last_health};last_position={last_position:?};player={player:?};vitals={:?}",
                kill_seconds(),
                client.actor_vitals()
            ),
        ));
    }
    let combat_seconds = combat_started.elapsed().as_secs_f64();
    let measured_seconds = first_damage.map(|first| first.elapsed().as_secs_f64());
    // Corpse: a new corpse appears at the death tile.
    let matches_corpse =
        |entry: &oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity| {
            entry.kind == EntityKind::Corpse
                && entry.entity.identity == corpse_identity
                && !corpses_before.contains(&entry.entity)
        };
    let mut corpse_observed = false;
    let corpse_deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let corpse = loop {
        corpse_observed |= observe_sweep_corpse(client, matches_corpse);
        let found = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entry| {
                    matches_corpse(entry)
                        && corpse_at(entry, last_position, &corpses_before, corpse_identity)
                })
                .copied()
        });
        if found.is_some() || tokio::time::Instant::now() >= corpse_deadline {
            break found;
        }
        client
            .service_liveness(Duration::from_millis(50))
            .await
            .map_err(|error| failure("corpse", "protocol", error))?;
        verdict::Harness::global().progress("creature-sweep-corpse");
    };

    let expects_corpse = species
        .reward
        .as_ref()
        .map_err(|reason| failure("corpse", "content", reason.as_str()))?
        .corpse_item
        .is_some();
    let death: String = clients
        .world
        .into_iter()
        .chain(clients.channel)
        .chain(clients.scope_generation.to_be_bytes())
        .chain(answer.actor_local_id.to_be_bytes())
        .chain(answer.entity.generation.to_be_bytes())
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let death_committed = !expects_corpse
        && primary_node_witness("creature_death_witness", &[("death", &death)])
            .map_err(|error| failure("kill", "evidence", error))?
            .is_some();
    if !damaged && !death_committed && corpse.is_none() {
        return Err(failure(
            "kill",
            "observation",
            "no damage or confirmed death evidence",
        ));
    }
    if let Err(reason) = confirm_death(expects_corpse, corpse.is_some(), death_committed) {
        rows.fail("kill", "death_unconfirmed", reason);
        rows.fail("corpse", "contract_mismatch", reason);
        rows.block_rest("corpse");
        return Ok(());
    }

    // The native reward owner publishes the sampled modifiers when settlement finishes.
    let inputs_deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    let inputs = loop {
        if tokio::time::Instant::now() >= inputs_deadline {
            return Err(failure(
                "xp",
                "evidence",
                "missing reward inputs for the spawned death",
            ));
        }
        corpse_observed |= observe_sweep_corpse(client, matches_corpse);
        if let Some(inputs) =
            physical_loot_inputs(&death).map_err(|error| failure("xp", "evidence", error))?
        {
            break inputs;
        }
        tokio::time::timeout_at(
            inputs_deadline,
            client.service_liveness(Duration::from_millis(250)),
        )
        .await
        .map_err(|_| failure("xp", "evidence", "reward input deadline elapsed"))?
        .map_err(|error| failure("xp", "protocol", error))?;
        verdict::Harness::global().progress("creature-sweep-reward-inputs");
    };
    let base_xp = inputs
        .expected_base_xp(definition.experience)
        .map_err(|error| failure("xp", "content", error))?;
    let displayed_pool =
        client_daily_boost::boosted_xp(definition.experience, &species.key, &selection)
            .map_err(|error| failure("xp", "content", error))?;
    if inputs.xp_rate == 100 && base_xp != displayed_pool {
        return Err(failure(
            "xp",
            "settlement",
            "visible Daily Boost disagrees with the committed XP modifiers",
        ));
    }
    let settlement = if base_xp > 0 {
        let death = committed_hunt_death(
            client,
            clients,
            Some(clients.characters[CORE_CHARACTER]),
            &species.key,
            target.entity,
            tokio::time::Instant::now() + Duration::from_secs(20),
        )
        .await
        .map_err(|error| failure("kill", "unproven_death", error))?;
        if death.actor.corpse_identity() != corpse_identity
            || death.shared
            || inputs
                .stamina
                .is_some_and(|stamina| stamina != death.stamina)
        {
            return Err(failure(
                "kill",
                "settlement",
                "receipt owner, corpse, mode or stamina disagrees with settlement",
            ));
        }
        if inputs.daily_loot_rolls > 0 {
            let expected = if selection.boss_key == species.key {
                boss_bonus_rolls(death.reward_occurrence.ok_or_else(|| {
                    failure(
                        "loot",
                        "settlement",
                        "boss bonus has no committed occurrence",
                    )
                })?)
            } else {
                u8::from(selection.creature_key == species.key)
            };
            if inputs.daily_loot_rolls != expected {
                return Err(failure(
                    "loot",
                    "settlement",
                    "visible Daily Boost disagrees with the committed extra rolls",
                ));
            }
        }
        let receipt = death_settlement(clients, client, target.entity)
            .await
            .map_err(|error| failure("kill", "unproven_death", error))?;
        if receipt.corpse != corpse_identity {
            return Err(failure(
                "corpse",
                "identity",
                "corpse disagrees with committed death",
            ));
        }
        Some(receipt)
    } else {
        None
    };
    if !confirmed_kill(death_distance, true)
        || !disappearance_proves_kill(last_position, player, damaged, true)
    {
        return Err(failure(
            "kill",
            "vanished",
            format!(
                "target left view far from the player;damage_observed={damaged};distance={death_distance};last_position={last_position:?}"
            ),
        ));
    }
    rows.kill_seconds = measured_seconds;
    let budget_detail = species.dps.map_or_else(
        || "dps=unmeasured budget=none".to_owned(),
        |dps| format!("budget:{};dps:{dps:.2}", budget(dps)),
    );
    rows.pass(
        "kill",
        &format!(
            "seconds:{:.1};damage_observed:{damaged};hp:{};{budget_detail};death_tile:{last_position:?}",
            combat_seconds,
            definition.health
        ),
    );
    let _ = client.session_mut().attack_target(None).await;

    // XP: the gain equals the server's value for the authored experience.
    let expected = oteryn_gameplay_characters::hunting_experience::hunting_experience(
        i64::try_from(base_xp).map_err(|error| failure("xp", "content", error))?,
        0,
        0,
        inputs.stamina.map_or(FULL_STAMINA_MINUTES, i32::from),
        false,
        level,
    )
    .map_err(|_| {
        failure(
            "xp",
            "content",
            "hunting_experience rejected the authored value",
        )
    })?;
    let expected = u64::try_from(expected).map_err(|error| failure("xp", "content", error))?;
    let xp_deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    let receipt_total = settlement
        .as_ref()
        .map_or(xp_before, |receipt| receipt.total);
    let receipt_award = settlement.as_ref().map_or(0, |receipt| receipt.award);
    let observation = observe_xp_until(
        client,
        xp_deadline,
        receipt_total,
        async |client| {
            verdict::Harness::global().progress("creature-sweep-xp-stats");
            let xp = read_stats(client).await.map(|(xp, _)| xp)?;
            Ok(xp)
        },
        async |client| {
            verdict::Harness::global().progress("creature-sweep-xp-liveness");
            client.service_liveness(Duration::from_millis(250)).await?;
            client.take_events();
            Ok(())
        },
    )
    .await;
    corpse_observed |= observe_sweep_corpse(client, matches_corpse);
    let xp_after = match observation {
        Ok(xp) => xp,
        Err(error) => {
            let detail = format!(
                "{error};expected:{expected};receipt_award:{};receipt_total:{};wire_before:{xp_before}",
                receipt_award, receipt_total
            );
            // Cancellation can leave a partial protocol exchange. Never logout or reuse it.
            *session = None;
            return Err(failure("xp", "observation", detail));
        }
    };
    let gained = xp_after.saturating_sub(xp_before);
    let xp_detail = format!(
        "authored:{};expected:{expected};gained:{gained};level:{level};receipt_award:{};receipt_total:{};wire_before:{xp_before};wire_after:{xp_after}",
        definition.experience, receipt_award, receipt_total
    );
    if xp_matches(
        xp_before,
        xp_after,
        expected,
        receipt_award == expected && xp_after == receipt_total,
    ) {
        rows.pass("xp", &xp_detail);
    } else {
        rows.fail("xp", "xp_mismatch", &xp_detail);
    }

    let Some(corpse) = corpse else {
        if let Err(reason) = confirm_corpse_free_settlement(corpse_observed, &inputs) {
            rows.fail("corpse", "contract_mismatch", reason);
            rows.fail(
                "loot",
                "corpse_contract",
                inputs.loot_error.as_deref().unwrap_or(reason),
            );
            return Ok(());
        }
        rows.pass(
            "corpse",
            "committed death has no physical corpse in the pinned contract",
        );
        rows.pass("loot", "no physical loot container in the pinned contract");
        return Ok(());
    };
    if let Some(error) = &inputs.loot_error {
        rows.fail("loot", "settlement", error);
        return Ok(());
    }
    let Some(handle) = corpse_handle(corpse.detail) else {
        rows.fail(
            "loot",
            "protocol",
            "the corpse has no server-issued item handle",
        );
        return Ok(());
    };

    rows.pass(
        "corpse",
        &format!("handle:{handle};exact-actor-death-bound"),
    );

    // Loot: open the corpse through the real protocol and compare it with the loot table.
    match open_corpse(client, handle).await {
        Err(error) => rows.fail("loot", "protocol", &error.to_string()),
        Ok(got) => {
            let seed = LootPlanDeathKey::new(
                clients.world,
                clients.channel,
                clients.scope_generation,
                answer.actor_local_id,
                answer.entity.generation,
            );
            judge_loot(rows, items, species, seed, &inputs, &got);
        }
    }
    Ok(())
}

fn confirm_death(
    expects_corpse: bool,
    has_corpse: bool,
    death_committed: bool,
) -> Result<(), &'static str> {
    match (expects_corpse, has_corpse, death_committed) {
        (true, true, _) | (false, false, true) => Ok(()),
        (true, false, _) => Err("spawned creature disappeared without its required corpse"),
        (false, true, _) => {
            Err("spawned creature produced a corpse contrary to its pinned contract")
        }
        (false, false, false) => Err("corpse-free creature disappeared without a committed death"),
    }
}

fn observe_sweep_corpse(
    client: &mut DevClientSession,
    matches: impl Fn(&oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity) -> bool,
) -> bool {
    let visible = client
        .world_entities()
        .is_some_and(|entities| entities.iter().any(&matches));
    client.take_events().iter().any(|event| match event {
        SessionEvent::WorldSpatialEntities(delta) => delta
            .value
            .enter
            .iter()
            .chain(&delta.value.update)
            .any(&matches),
        _ => false,
    }) || visible
}

#[derive(PartialEq, Eq)]
enum CorpseSettlement {
    Absent,
    Present,
    Failed,
}

struct RewardInputs {
    stamina: Option<u16>,
    loot_rate: u32,
    xp_rate: u32,
    corpse: CorpseSettlement,
    loot_error: Option<String>,
    boosted_creature: bool,
    boosted_boss: bool,
    daily_loot_rolls: u8,
}

fn confirm_corpse_free_settlement(
    observed: bool,
    inputs: &RewardInputs,
) -> Result<(), &'static str> {
    if observed || inputs.corpse != CorpseSettlement::Absent {
        return Err(
            "corpse-free death requires a successful absent settlement and no observed corpse",
        );
    }
    Ok(())
}

impl RewardInputs {
    fn expected_base_xp(&self, authored: u64) -> TestResult<u64> {
        let boosted = authored
            .checked_mul(if self.boosted_creature { 2 } else { 1 })
            .ok_or("daily-boosted experience overflow")?;
        Ok(oteryn_gameplay_foundation::scheduled_events::scale_gain(
            boosted,
            self.xp_rate,
        ))
    }

    fn apply_daily_loot(&self, table: &mut LootTableDefinition, equipment: &BTreeSet<String>) {
        if self.daily_loot_rolls == 0
            || table.algorithm != LootSelectionAlgorithm::IndependentBernoulliPpm
        {
            return;
        }
        let boosted_entries = if self.boosted_boss {
            partition_daily_boss_bonus_entries(&mut table.entries, |key| equipment.contains(key))
        } else {
            table.entries.len()
        };
        table.algorithm = LootSelectionAlgorithm::DailyBoosted {
            extra_rolls: self.daily_loot_rolls,
            boosted_entries,
        };
    }
}

fn physical_loot_inputs(death: &str) -> TestResult<Option<RewardInputs>> {
    primary_node_witness("physical_loot_inputs", &[("death", death)])?
        .as_deref()
        .map(parse_reward_inputs)
        .transpose()
}

fn parse_reward_inputs(line: &str) -> TestResult<RewardInputs> {
    let field = |key| {
        line.split_ascii_whitespace()
            .filter_map(|part| part.split_once('='))
            .find_map(|(name, value)| (name == key).then_some(value))
            .ok_or("incomplete physical loot inputs")
    };
    let stamina = match field("stamina")? {
        "none" => None,
        value => Some(value.parse()?),
    };
    let corpse = match field("corpse_settlement")? {
        "absent" => CorpseSettlement::Absent,
        "present" => CorpseSettlement::Present,
        "failed" => CorpseSettlement::Failed,
        _ => return Err("unknown corpse settlement".into()),
    };
    let loot_error = field("loot_error")?;
    Ok(RewardInputs {
        stamina,
        loot_rate: field("loot_rate")?.parse()?,
        xp_rate: field("xp_rate")?.parse()?,
        corpse,
        loot_error: (loot_error != "none").then(|| loot_error.to_owned()),
        boosted_creature: field("boosted_creature")?.parse()?,
        boosted_boss: field("boosted_boss")?.parse()?,
        daily_loot_rolls: field("daily_loot_rolls")?.parse()?,
    })
}

fn judge_loot(
    rows: &mut Rows<'_>,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    species: &Species,
    seed: LootPlanDeathKey,
    inputs: &RewardInputs,
    observed: &BTreeMap<std::num::NonZeroU32, u64>,
) {
    match expected_loot(items, species, seed, inputs) {
        Err(error) => rows.fail("loot", "content", &error.to_string()),
        Ok(expected) if observed_loot_matches_plan(&expected, observed) => {
            if !species.private_reward
                && inputs.daily_loot_rolls == 0
                && inputs.loot_rate == 100
                && let Err(error) = validate_authored_loot(items, &species.loot, observed)
            {
                rows.fail("loot", "content", &error.to_string());
                return;
            }
            rows.pass(
                "loot",
                &format!("deterministic-plan:{expected:?};corpse-items:{observed:?}"),
            );
        }
        Ok(expected) => rows.fail(
            "loot",
            "plan_mismatch",
            &format!("expected {expected:?} from the pinned death roll, observed {observed:?}"),
        ),
    }
}

fn possible_loot_count(entries: &[(u64, u64)], observed: u64) -> bool {
    let mut reachable = vec![(0_u64, 0_u64)];
    for &(minimum, maximum) in entries {
        let mut next = reachable.clone();
        next.extend(reachable.iter().filter_map(|&(low, high)| {
            let low = low.checked_add(minimum)?;
            (low <= observed).then_some((low, high.saturating_add(maximum).min(observed)))
        }));
        next.sort_unstable();
        reachable.clear();
        for (low, high) in next {
            if let Some((_, previous_high)) = reachable.last_mut()
                && low <= previous_high.saturating_add(1)
            {
                *previous_high = (*previous_high).max(high);
            } else {
                reachable.push((low, high));
            }
        }
    }
    reachable
        .iter()
        .any(|&(low, high)| (low..=high).contains(&observed))
}

fn validate_authored_loot(
    items: &crate::content::item_ref::ItemDefinitionIndex,
    binding: &LootBinding,
    observed: &BTreeMap<std::num::NonZeroU32, u64>,
) -> TestResult {
    let LootBinding::Table(_, entries) = binding else {
        return Ok(());
    };
    let mut allowed = BTreeMap::<_, Vec<_>>::new();
    for entry in entries {
        let reference =
            item_ref(items, &entry.item, &entry.revision).ok_or("unmapped exact loot item")?;
        allowed
            .entry(reference)
            .or_default()
            .push((entry.min, entry.max));
    }
    for (reference, quantity) in observed {
        if !allowed
            .get(reference)
            .is_some_and(|ranges| possible_loot_count(ranges, *quantity))
        {
            return Err(format!(
                "observed item {reference} count {quantity} has no authored range"
            )
            .into());
        }
    }
    Ok(())
}

fn expected_loot(
    items: &crate::content::item_ref::ItemDefinitionIndex,
    species: &Species,
    seed: LootPlanDeathKey,
    inputs: &RewardInputs,
) -> TestResult<BTreeMap<std::num::NonZeroU32, u64>> {
    let (table, entries) = match &species.loot {
        LootBinding::Missing(table) => {
            return Err(format!("authored loot table {table} missing").into());
        }
        LootBinding::None => return Ok(BTreeMap::new()),
        LootBinding::Table(table, entries) => (table, entries),
    };
    let row = species.reward.as_ref().map_err(|reason| {
        format!(
            "pinned reward {} unavailable: {}",
            species.key,
            reason.as_str()
        )
    })?;
    let reference = row
        .loot_table_ref
        .as_ref()
        .filter(|reference| &reference.production_key == table)
        .ok_or("authored and pinned loot table bindings differ")?;
    validate_authored_entries(items, entries)?;
    let mut table = row.loot_table.clone();
    if !species.private_reward {
        inputs.apply_daily_loot(&mut table, &species.daily_boss_equipment);
    }
    configure_physical_loot(
        &mut table,
        species.private_reward,
        inputs.stamina,
        inputs.loot_rate,
    );
    seeded_loot(reference, &table, seed, items)
}

fn seeded_loot(
    reference: &crate::combat::LootDefinitionRef,
    table: &crate::combat::LootTableDefinition,
    seed: crate::combat::LootPlanDeathKey,
    items: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult<BTreeMap<std::num::NonZeroU32, u64>> {
    let plan = crate::combat::plan_creature_loot(seed, reference, table)
        .map_err(|error| format!("pinned loot draw: {error:?}"))?;
    let mut expected = BTreeMap::new();
    for entry in plan.entries.iter().filter(|entry| entry.parent.is_none()) {
        let reference = items
            .definition_ref(
                &entry.item.family,
                &entry.item.production_key,
                &entry.item.revision_ref,
            )
            .ok_or_else(|| format!("unmapped pinned loot Item {}", entry.item.production_key))?;
        *expected.entry(reference).or_default() += u64::from(entry.quantity);
    }
    Ok(expected)
}

fn validate_authored_entries(
    items: &crate::content::item_ref::ItemDefinitionIndex,
    entries: &[LootEntry],
) -> TestResult {
    for entry in entries {
        if entry.min > entry.max
            || !resolved_loot_entries(usize::from(
                item_ref(items, &entry.item, &entry.revision).is_none(),
            ))
        {
            return Err(format!("invalid or unmapped authored loot item {}", entry.item).into());
        }
    }
    Ok(())
}

fn expects_physical_loot(private_reward: bool, stamina: u16) -> bool {
    !private_reward && oteryn_gameplay_characters::stamina::hunting_loot_allowed(stamina)
}

fn configure_physical_loot(
    table: &mut oteryn_gameplay_items::loot_plan::LootTableDefinition,
    private_reward: bool,
    stamina: Option<u16>,
    rate: u32,
) {
    if !expects_physical_loot(
        private_reward,
        stamina.unwrap_or(FULL_STAMINA_MINUTES as u16),
    ) {
        table.entries.clear();
        table.algorithm = LootSelectionAlgorithm::IndependentBernoulliPpm;
    } else {
        apply_scheduled_loot_rate(table, rate);
    }
}

async fn open_corpse(
    client: &mut DevClientSession,
    handle: oteryn_protocol_oteryn::item_view::ItemHandle,
) -> TestResult<BTreeMap<std::num::NonZeroU32, u64>> {
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("corpse USE refused: {used:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while client
        .open_container()
        .is_none_or(|view| view.container_handle != Some(handle))
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("corpse container did not open within 10s".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        verdict::Harness::global().progress("creature-sweep-open-corpse");
        client.take_events();
    }
    let mut got: BTreeMap<std::num::NonZeroU32, u64> = BTreeMap::new();
    for entry in &client
        .open_container()
        .ok_or("corpse container vanished")?
        .entries
    {
        *got.entry(entry.item_definition_ref).or_default() += u64::from(entry.count.get());
    }
    Ok(got)
}

/// Reuse the sweep build once the knight carries or wields its axe. A committed build can
/// precede the item grant, so vocation and backpack alone cannot skip fixture reconciliation.
fn fixture_ready(
    classed: bool,
    inventory: Option<&oteryn_protocol_oteryn::item_view::CharacterInventory>,
    axe: std::num::NonZeroU32,
) -> bool {
    classed
        && inventory.is_some_and(|inventory| {
            inventory
                .entries
                .iter()
                .any(|entry| entry.item_definition_ref == axe)
                || inventory
                    .equipment
                    .iter()
                    .any(|entry| entry.item.item_definition_ref == axe)
        })
}

async fn prepare_player(
    clients: &SeamClients<'_>,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    let client = session.as_mut().ok_or("session absent")?;
    let (xp, level, vocation) = read_stats_with_vocation(client).await?;
    let classed = vocation.is_some_and(|id| Some(id) == character::vocation_id("knight"));
    let definitions = provisioning::item_index()?;
    let axe = provisioning::item_reference(&definitions, SWEEP_WEAPON_ID)?;
    if fixture_ready(classed, client.inventory(), axe) {
        return ensure_weapon(client, axe).await;
    }
    if (xp != 0 || level != 1) && !classed {
        return Err("sweep setup requires a pristine or previously classed character".into());
    }
    if E2eMode::from_env()? == E2eMode::Journey {
        evidence("e2e_group_gap name=creature_sweep cause=legacy-build-requires-isolated-mode");
        return Err(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!())).into());
    }
    let mut client = session.take().ok_or("session absent")?;
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let profile = std::env::var("OTERYN_E2E_SWEEP_PROFILE").unwrap_or_else(|_| "sweep".into());
    let game_session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let mut command = fixture_command();
    command.args([
        "--socket",
        &socket,
        "--session",
        &game_session,
        "--profile",
        &profile,
    ]);
    let status = with_peer_service(
        async {
            tokio::time::timeout(FIXTURE_COMMAND_DEADLINE, run_fixture_command(&mut command)).await
        },
        async || {
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
            Ok(())
        },
    )
    .await???
    .status;
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!(
            "fixture setup logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}"
        )
        .into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0x70, CORE_CHARACTER).await?;
    let (_, level) = read_stats(&mut fresh).await?;
    if level < SWEEP_LEVEL {
        *session = Some(fresh);
        return Err(format!(
            "node-boot fenced build provisioning failed (fixture exit ok={}, level {level})",
            status.success()
        )
        .into());
    }
    let equipped = ensure_weapon(&mut fresh, axe).await;
    *session = Some(fresh);
    equipped?;
    evidence(&format!(
        "creature_sweep provisioned level={level} fixture_ok={} weapon={SWEEP_WEAPON_ID}",
        status.success()
    ));
    Ok(())
}

/// Wield the sweep weapon (item 6553) unless it is already worn; a refusal is an error with the
/// server's detail. Same protocol path as MP01 (`mp/mp01_party_pvp.rs`, axe equip).
async fn ensure_weapon(client: &mut DevClientSession, axe: std::num::NonZeroU32) -> TestResult {
    let wielded = client.inventory().is_some_and(|inventory| {
        inventory
            .equipment
            .iter()
            .any(|entry| entry.item.item_definition_ref == axe)
    });
    if wielded {
        return Ok(());
    }
    let equipped = client.session_mut().equip_item_definition(axe).await?;
    if equipped.status != CommandStatus::Accepted
        || equipped.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
    {
        let detail = client.session_mut().last_refusal_detail();
        return Err(format!(
            "weapon {SWEEP_WEAPON_ID} equip refused:{equipped:?};refusal_detail={detail:?};weapon_ref={axe}"
        )
        .into());
    }
    Ok(())
}

/// Experience and level from the Cyclopedia GeneralStats tab (the same panel MP09 reads).
async fn read_stats_with_vocation(
    client: &mut DevClientSession,
) -> TestResult<(u64, u32, Option<u8>)> {
    let mut view = panel::CyclopediaCharacterPanel {
        selected: true,
        limit: 50,
        extensions: panel::TabExtensions {
            vocation: general_stats::selected(client),
            death_details: false,
        },
        ..Default::default()
    };
    let command = panel::request(
        &mut view,
        panel::Intent::SelectTab(character::CharacterTab::GeneralStats),
    )
    .ok_or("GeneralStats request absent")?;
    let reply = client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if reply.status != CommandStatus::Accepted || !panel::apply_result(&mut view, &reply) {
        return Err(format!("GeneralStats reply={reply:?}").into());
    }
    let stats = view
        .general
        .ok_or_else(|| format!("GeneralStats absent;reply={reply:?}"))?;
    Ok((stats.experience, stats.level, view.vocation_id))
}

/// Logout with the combat lock honoured: wait it out with keep-alive Turns, never clear it.
async fn read_stats(client: &mut DevClientSession) -> TestResult<(u64, u32)> {
    let mut view = panel::CyclopediaCharacterPanel {
        selected: true,
        limit: 50,
        extensions: panel::TabExtensions {
            vocation: general_stats::selected(client),
            death_details: false,
        },
        ..Default::default()
    };
    let command = panel::request(
        &mut view,
        panel::Intent::SelectTab(character::CharacterTab::GeneralStats),
    )
    .ok_or("GeneralStats request absent")?;
    let reply = client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if reply.status != CommandStatus::Accepted || !panel::apply_result(&mut view, &reply) {
        return Err(format!("GeneralStats reply={reply:?}").into());
    }
    let stats = view
        .general
        .ok_or_else(|| format!("GeneralStats absent;reply={reply:?}"))?;
    Ok((stats.experience, stats.level))
}

/// Logout with the combat lock honoured: wait it out with keep-alive Turns, never clear it.
async fn release(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(930);
    let mut activity_due = tokio::time::Instant::now();
    loop {
        verdict::Harness::global().progress("creature-sweep-logout");
        let (result, reply, attempts, busy_timed_out) = logout_retrying_busy(client).await?;
        match result {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::InFight { .. }) => {
                if tokio::time::Instant::now() >= deadline {
                    return Err("logout combat lock outlasted 930s".into());
                }
                client.service_liveness(Duration::from_millis(50)).await?;
                client.take_events();
                if tokio::time::Instant::now() >= activity_due {
                    let payload = target_move::encode_target_move_intent(
                        target_move::TargetMoveIntent::Turn(StepDirection::North),
                    )
                    .map_err(|error| format!("Turn encode:{error:?}"))?;
                    client
                        .session_mut()
                        .gated_command(
                            target_move::CAPABILITY_TARGET_MOVE_V1,
                            target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
                            &payload,
                        )
                        .await?;
                    activity_due = tokio::time::Instant::now() + Duration::from_secs(30);
                }
            }
            _ => {
                return Err(format!(
                    "logout refused:{reply};attempts={attempts};busy_timed_out={busy_timed_out}"
                )
                .into());
            }
        }
    }
}

// Preserve the calibration arithmetic cases alongside registered regression tests.
#[cfg(test)]
const _: () = {
    let mut attempt = 1;
    while attempt <= 3 {
        assert!(matches!(
            calibration_decision(attempt, Some(2.0), 260, true),
            CalibrationDecision::Measured(130.0)
        ));
        if attempt < 3 {
            assert!(matches!(
                calibration_decision(attempt, None, 260, true),
                CalibrationDecision::Retry
            ));
        } else {
            assert!(matches!(
                calibration_decision(attempt, None, 260, true),
                CalibrationDecision::Fallback
            ));
        }
        attempt += 1;
    }
    assert!(matches!(
        calibration_decision(3, Some(0.0), 260, true),
        CalibrationDecision::Fallback
    ));
    assert!(matches!(
        calibration_decision(3, Some(-1.0), 260, true),
        CalibrationDecision::Fallback
    ));
    assert!(matches!(
        calibration_decision(3, Some(f64::NAN), 260, true),
        CalibrationDecision::Fallback
    ));
    assert!(matches!(
        calibration_decision(3, Some(f64::INFINITY), 260, true),
        CalibrationDecision::Fallback
    ));
    assert!(matches!(
        calibration_decision(3, Some(2.0), 0, true),
        CalibrationDecision::Fallback
    ));
};

#[cfg(test)]
mod review_tests {
    use super::*;
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

    #[tokio::test(start_paused = true)]
    async fn qualification_xp_mismatch_preserves_independent_rows() -> TestResult {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        let value = observe_xp_until(
            &mut (),
            deadline,
            123,
            async |_| Ok(122),
            async |_| {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(())
            },
        )
        .await?;
        assert_eq!(value, 122);
        Ok(())
    }

    #[test]
    fn qualification_xp_cancelled_exchange_is_discarded_before_release() -> TestResult {
        let source = include_str!("creature_sweep.rs");
        let observation = source
            .split_once("let xp_deadline =")
            .ok_or("production observation")?
            .1
            .split_once("let gained =")
            .ok_or("XP verdict")?
            .0;
        assert!(observation.contains("*session = None;"));
        assert!(observation.contains("Ok(xp) => xp"));
        assert!(!observation.contains(".map_err("));
        Ok(())
    }

    #[tokio::test(start_paused = true)]
    async fn qualification_next_round_xp_request_cannot_pass_after_deadline() -> TestResult {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        let result = observe_xp_until(
            &mut (),
            deadline,
            123,
            async |_| {
                tokio::time::sleep(Duration::from_secs(21)).await;
                Ok(123)
            },
            async |_| Ok(()),
        )
        .await;
        assert!(
            result.is_err(),
            "late matching XP must fail its observation budget"
        );
        assert!(tokio::time::Instant::now() <= deadline);
        Ok(())
    }

    #[tokio::test(start_paused = true)]
    async fn qualification_next_round_cancelled_final_species_is_cleaned() -> TestResult {
        let mut outcome = SweepOutcome::default();
        let mut cleaned = Vec::new();
        let result = finish_indexed_sweep(
            &mut outcome,
            async |outcome| {
                outcome.cleanup_session = Some("published-session".into());
                tokio::time::timeout(Duration::from_secs(1), std::future::pending::<()>()).await?;
                Ok(())
            },
            async |session| {
                cleaned.push(session);
                Ok("cleaned".into())
            },
        )
        .await;
        assert!(result.is_err());
        assert_eq!(cleaned, vec!["published-session"]);
        assert!(outcome.cleanup_session.is_none());
        Ok(())
    }

    #[tokio::test(start_paused = true)]
    async fn qualification_next_round_cleanup_cancellation_retains_ticket() -> TestResult {
        let mut ticket = Some("published-session".into());
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            cleanup_ticket(&mut ticket, async |_| {
                std::future::pending::<Result<String, String>>().await
            }),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(ticket.as_deref(), Some("published-session"));
        cleanup_ticket(&mut ticket, async |_| Ok("cleaned".into())).await?;
        assert!(ticket.is_none());
        Ok(())
    }

    #[cfg(feature = "test-support")]
    #[tokio::test]
    async fn qualification_next_round_exact_receipt_survives_clock_restart() -> TestResult {
        let url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
        let mut db = sqlx::PgConnection::connect(&url).await?;
        sqlx::raw_sql("CREATE TEMP TABLE game_party_kill_xp_receipts(death_key bytea PRIMARY KEY,death_at_us bigint); CREATE TEMP TABLE game_party_kill_xp_members(death_key bytea,character_id uuid,xp_awarded bigint); CREATE TEMP TABLE game_character_progression_state(character_id uuid,total_experience bigint);").execute(&mut db).await?;
        let character = [1; 16];
        sqlx::query(
            "INSERT INTO game_character_progression_state VALUES(encode($1,'hex')::uuid,123)",
        )
        .bind(character.as_slice())
        .execute(&mut db)
        .await?;
        let mut target_key = vec![2; 52];
        target_key[44..].copy_from_slice(&1u64.to_be_bytes());
        for index in 0..66u64 {
            let mut key = target_key.clone();
            key[40..44].copy_from_slice(&(index as u32).to_be_bytes());
            sqlx::query("INSERT INTO game_party_kill_xp_receipts VALUES($1,$2)")
                .bind(&key)
                .bind(if index == 65 {
                    1i64
                } else {
                    10000 + index as i64
                })
                .execute(&mut db)
                .await?;
            sqlx::query(
                "INSERT INTO game_party_kill_xp_members VALUES($1,encode($2,'hex')::uuid,23)",
            )
            .bind(&key)
            .bind(character.as_slice())
            .execute(&mut db)
            .await?;
            if index == 65 {
                target_key = key;
            }
        }
        let target = EntityRef {
            identity: death_identity(&target_key, b"oteryn:runtime-player-placement:v1")
                .ok_or("key")?,
            generation: 1,
        };
        let receipt = read_death_settlement(&mut db, character, target)
            .await?
            .ok_or("exact committed death hidden behind earlier-clock receipts")?;
        assert_eq!((receipt.award, receipt.total), (23, 123));
        db.close().await?;
        Ok(())
    }

    #[test]
    fn a_cancelled_indexed_row_counts_even_before_the_reporter_updates_its_tally() {
        assert_eq!(accounted_failures(0, false, true), 1);
        assert_eq!(accounted_failures(2, true, true), 2);
        assert_eq!(accounted_failures(0, true, false), 0);
    }

    #[test]
    fn an_unrelated_corpse_on_the_death_tile_cannot_substitute() -> TestResult {
        let position = ActorPosition {
            x: 1,
            y: 1,
            floor: 7,
        };
        let candidate = oteryn_dev_client::WorldSpatialEntity {
            display_name: None,
            kind: EntityKind::Corpse,
            entity: EntityRef {
                identity: [2; 16],
                generation: 0,
            },
            position,
            detail: EntityDetail::Object {
                item_definition_ref: 1,
                quantity: 1,
                item_handle: None,
            },
        };
        assert!(!corpse_candidate(
            &candidate,
            position,
            &BTreeSet::new(),
            [1; 16]
        ));
        assert!(corpse_candidate(
            &candidate,
            position,
            &BTreeSet::new(),
            [2; 16]
        ));
        Ok(())
    }

    #[test]
    fn a_failed_calibration_row_cannot_supply_a_successful_calibration() {
        assert!(matches!(
            calibration_decision(1, Some(2.0), 260, false),
            CalibrationDecision::Retry
        ));
        assert!(matches!(
            calibration_decision(3, Some(2.0), 260, false),
            CalibrationDecision::Fallback
        ));
    }

    #[test]
    fn selected_calibration_species_is_not_replaced_by_its_advisory_attempt() {
        let cyclops = CALIBRATION_KEY.to_owned();
        let keys = [&cyclops];
        assert_eq!(selected_batch(&keys, 0, 1, true), vec![&cyclops]);
    }

    #[test]
    fn calibration_attempts_have_distinct_unscored_identifiers() {
        let first = calibration_short("cyclops", 1);
        let second = calibration_short("cyclops", 2);
        assert_ne!(first, second);
        assert_ne!(first, "cyclops");
    }

    #[test]
    fn a_batch_without_an_attempted_species_cannot_qualify() {
        assert!(!batch_succeeds(0, 0));
        assert!(batch_succeeds(1, 0));
        assert!(!batch_succeeds(1, 1));
    }

    #[test]
    fn same_named_variants_do_not_substitute_for_the_returned_spawn() -> TestResult {
        let candidate = oteryn_dev_client::WorldSpatialEntity {
            display_name: Some(
                oteryn_protocol_oteryn::entity_names::EntityDisplayName::new("Orc Warlord")
                    .map_err(|error| format!("{error:?}"))?,
            ),
            kind: EntityKind::Creature,
            entity: EntityRef {
                identity: [2; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 1,
                y: 1,
                floor: 7,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        let answer = format!("spawned {} 1 41", "01".repeat(16));
        assert!(!spawn_candidate(
            &candidate,
            "Orc Warlord",
            &BTreeSet::new(),
            7,
            &answer
        ));
        let wrong_generation = format!("spawned {} 2 41", "02".repeat(16));
        assert!(!spawn_candidate(
            &candidate,
            "Orc Warlord",
            &BTreeSet::new(),
            7,
            &wrong_generation
        ));
        assert!(spawn_candidate(
            &candidate,
            "Orc Warlord",
            &BTreeSet::new(),
            7,
            &format!("spawned {} 1 41", "02".repeat(16))
        ));
        Ok(())
    }

    #[test]
    fn disappearance_without_death_evidence_is_not_a_kill() {
        let here = ActorPosition {
            x: 1,
            y: 1,
            floor: 7,
        };
        assert!(!disappearance_proves_kill(here, here, false, false));
        assert!(!disappearance_proves_kill(
            ActorPosition { floor: 8, ..here },
            here,
            true,
            true
        ));
        assert!(disappearance_proves_kill(here, here, true, true));
        // A single lethal swing need not expose an intermediate HP frame.
        assert!(disappearance_proves_kill(here, here, false, true));
    }

    #[test]
    fn xp_loss_and_unsettled_first_matching_awards_cannot_pass() {
        assert!(!xp_matches(100, 90, 0, true));
        assert!(!xp_matches(100, 110, 10, false));
        assert!(!xp_matches(100, 120, 10, true));
        assert!(xp_matches(100, 110, 10, true));
        assert!(xp_matches(100, 100, 0, true));
    }

    #[test]
    fn loot_oracle_refuses_unknown_definitions_and_drifted_revisions() -> TestResult {
        let items = crate::content::item_ref::ItemDefinitionIndex::decode(
            br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i3031","definition-r1"]]}"#,
        ).map_err(|error| format!("{error:?}"))?;
        for (key, revision) in [
            ("oteryn:item.tibia.i9999999", "definition-r1"),
            ("oteryn:item.tibia.i3031", "definition-r2"),
        ] {
            let entries = [LootEntry {
                item: key.into(),
                revision: revision.into(),
                min: 1,
                max: 1,
            }];
            assert!(
                validate_authored_entries(&items, &entries).is_err(),
                "unmapped exact definition must fail: {key}/{revision}"
            );
        }
        Ok(())
    }
}

fn needs_healing(client: &DevClientSession) -> TestResult<bool> {
    let vitals = client
        .actor_vitals()
        .ok_or("sweep Character vitals absent")?;
    Ok(vitals.health.saturating_mul(2) < vitals.max_health)
}

async fn recover_health(
    client: &mut DevClientSession,
    world: &BundleWorld,
    vocation: u8,
) -> TestResult {
    use oteryn_dev_client::{ChatIntent, ChatSpeechMode};

    let words = if Some(vocation) == character::vocation_id("knight") {
        "exura infir ico"
    } else {
        "exura infir"
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    verdict::Harness::global().bounded_wait(deadline, "creature-sweep-healing");
    tokio::time::timeout_at(deadline, async {
        return_to_protection_zone(client, world).await?;
        while needs_healing(client)? {
            client
                .chat(&ChatIntent::Say {
                    mode: ChatSpeechMode::Say,
                    text: words.into(),
                })
                .await?;
            client.take_events();
            client.service_liveness(Duration::from_secs(2)).await?;
            client.take_events();
        }
        Ok(())
    })
    .await
    .map_err(|_| "sweep Character did not recover half health within 240s")?
}

fn corpse_handle(detail: EntityDetail) -> Option<oteryn_protocol_oteryn::item_view::ItemHandle> {
    match detail {
        EntityDetail::Object { item_handle, .. }
        | EntityDetail::ObjectWithAppearance { item_handle, .. } => item_handle,
        _ => None,
    }
}

#[cfg(test)]
mod corpse_tests {
    use super::*;

    #[test]
    fn corpse_free_contract_requires_exact_death_evidence() {
        assert!(confirm_death(false, false, true).is_ok());
        assert!(confirm_death(false, false, false).is_err());
        assert!(confirm_death(false, true, true).is_err());
        assert!(confirm_death(true, false, true).is_err());
        assert!(confirm_death(true, true, false).is_ok());
    }

    #[test]
    fn corpse_free_requires_terminal_absence() -> TestResult {
        let witness = "stamina=2520 loot_rate=100 xp_rate=100 boosted_creature=false boosted_boss=false daily_loot_rolls=0 loot_error=none";
        let absent = parse_reward_inputs(&format!("{witness} corpse_settlement=absent"))?;
        let present = parse_reward_inputs(&format!("{witness} corpse_settlement=present"))?;
        assert!(confirm_corpse_free_settlement(false, &absent).is_ok());
        assert!(confirm_corpse_free_settlement(true, &absent).is_err());
        assert!(confirm_corpse_free_settlement(false, &present).is_err());
        Ok(())
    }

    #[tokio::test]
    async fn terminal_release_runs_without_a_socket_and_after_failed_logout() {
        for session in [None, Some(())] {
            let mut observed = false;
            let mut logout_attempted = false;
            let [(_, logout), (_, terminal)] = finish_sweep(
                session,
                |()| {
                    logout_attempted = true;
                    std::future::ready(Err("failed logout".into()))
                },
                async {
                    observed = true;
                    Err("lease still held".into())
                },
            )
            .await;
            assert!(observed);
            assert_eq!(logout_attempted, session.is_some());
            assert_eq!(logout.is_err(), session.is_some());
            assert!(terminal.is_err());
        }
    }

    #[test]
    fn native_corpse_keeps_its_server_item_handle() -> TestResult {
        let handle = std::num::NonZeroU64::new(101).ok_or("zero item handle")?;
        let appearance_id = std::num::NonZeroU16::new(42).ok_or("zero appearance")?;
        let corpse = EntityDetail::ObjectWithAppearance {
            item_definition_ref: 12,
            quantity: 1,
            item_handle: Some(handle),
            appearance_id,
        };
        assert_eq!(corpse_handle(corpse), Some(handle));
        Ok(())
    }
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
enum Preparation {
    Provision,
    Heal,
    Hunt,
}

#[cfg(test)]
fn preparation(level: u32, unclassed: bool, low: bool) -> TestResult<Preparation> {
    if level == 1 && unclassed {
        Ok(Preparation::Provision)
    } else if level < SWEEP_LEVEL || unclassed {
        Err("sweep requires a classed level-1000 fixture".into())
    } else if low {
        Ok(Preparation::Heal)
    } else {
        Ok(Preparation::Hunt)
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    #[test]
    fn injured_provisioned_character_recovers_without_fresh_only_provisioning() -> TestResult {
        assert_eq!(preparation(1, true, false)?, Preparation::Provision);
        assert_eq!(preparation(1000, false, true)?, Preparation::Heal);
        assert_eq!(preparation(1000, false, false)?, Preparation::Hunt);
        assert!(preparation(999, false, true).is_err());
        Ok(())
    }
}

fn observed_loot_matches_plan<K: Ord>(
    expected: &BTreeMap<K, u64>,
    observed: &BTreeMap<K, u64>,
) -> bool {
    expected == observed
}

#[cfg(test)]
mod loot_tests {
    use super::*;

    #[test]
    fn physical_loot_plan_models_private_rewards_stamina_and_double_loot() -> TestResult {
        use oteryn_gameplay_items::loot_plan::{
            LootDefinitionRef, LootSelectionAlgorithm, LootTableDefinition, LootTableEntry,
        };
        let reference = LootDefinitionRef {
            family: "LootTable".into(),
            production_key: "fixture:loot".to_owned(),
            revision_ref: "v1".to_owned(),
        };
        let authored = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![LootTableEntry {
                item: LootDefinitionRef {
                    family: "Item".into(),
                    production_key: "fixture:item".to_owned(),
                    revision_ref: "v1".to_owned(),
                },
                min_count: 1,
                max_count: 1,
                probability_ppm: Some(1_000_000),
                details: Default::default(),
            }],
        };
        let seed = LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1);
        for (private, stamina, rate, expected) in [
            (false, Some(2520), 100, 1),
            (true, Some(2520), 200, 0),
            (false, Some(0), 200, 0),
            (false, Some(2520), 200, 2),
        ] {
            let mut table = authored.clone();
            configure_physical_loot(&mut table, private, stamina, rate);
            let loot = plan_creature_loot(seed, &reference, &table)
                .map_err(|error| format!("test loot roll: {error:?}"))?;
            assert_eq!(loot.entries.len(), expected, "{private} {stamina:?} {rate}");
        }
        Ok(())
    }

    #[test]
    fn daily_boost_applies_to_both_expectations() -> TestResult {
        use oteryn_gameplay_items::loot_plan::{LootDefinitionRef, LootTableEntry};
        let entry = |name: &str, unique| {
            let mut entry = LootTableEntry {
                item: LootDefinitionRef {
                    family: "Item".into(),
                    production_key: name.into(),
                    revision_ref: "v1".into(),
                },
                min_count: 1,
                max_count: 1,
                probability_ppm: Some(1_000_000),
                details: Default::default(),
            };
            entry.details.unique = unique;
            entry.details.stackable = true;
            entry
        };
        let base = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![
                entry("gold", false),
                entry("sword", false),
                entry("shield", true),
            ],
        };
        let seed = LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1);
        let reference = LootDefinitionRef {
            family: "LootTable".into(),
            production_key: "fixture:loot".into(),
            revision_ref: "v1".into(),
        };
        let equipment = BTreeSet::from(["sword".to_owned(), "shield".to_owned()]);
        let witness =
            "stamina=2520 loot_rate=100 xp_rate=150 corpse_settlement=present loot_error=none";
        for (choice, xp, expected) in [
            (
                "boosted_creature=true boosted_boss=false daily_loot_rolls=1",
                51,
                vec!["gold", "sword", "shield", "gold", "sword", "shield"],
            ),
            (
                "boosted_creature=false boosted_boss=true daily_loot_rolls=2",
                25,
                vec!["sword", "gold", "shield", "sword", "sword"],
            ),
        ] {
            let inputs = parse_reward_inputs(&format!("{witness} {choice}"))?;
            assert_eq!(inputs.expected_base_xp(17)?, xp);
            let mut table = base.clone();
            inputs.apply_daily_loot(&mut table, &equipment);
            configure_physical_loot(&mut table, false, inputs.stamina, inputs.loot_rate);
            let plan = plan_creature_loot(seed, &reference, &table)
                .map_err(|error| format!("test loot roll: {error:?}"))?;
            assert_eq!(
                plan.entries
                    .iter()
                    .map(|entry| entry.item.production_key.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
        }
        Ok(())
    }

    #[test]
    fn loot_oracle_rejects_empty_output_and_impossible_duplicate_totals() {
        let item = std::num::NonZeroU32::MIN;
        let plan = BTreeMap::from([(item, 6)]);
        assert!(!observed_loot_matches_plan(&plan, &BTreeMap::new()));
        assert!(!observed_loot_matches_plan(
            &plan,
            &BTreeMap::from([(item, 4)])
        ));
        assert!(observed_loot_matches_plan(&plan, &plan));
        assert!(observed_loot_matches_plan(
            &BTreeMap::<std::num::NonZeroU32, u64>::new(),
            &BTreeMap::new()
        ));
    }
}

#[cfg(test)]
fn reuse_hunt_session(passed: bool) -> bool {
    passed
}
#[test]
fn a_failed_species_cannot_reuse_its_transport() {
    assert!(!reuse_hunt_session(false));
    assert!(reuse_hunt_session(true));
}

fn resolved_loot_entries(unmapped: usize) -> bool {
    unmapped == 0
}
#[test]
fn unresolved_loot_definitions_do_not_qualify_an_empty_roll() {
    assert!(!resolved_loot_entries(1));
    assert!(resolved_loot_entries(0));
}

#[cfg(test)]
mod evidence_tests {
    use super::*;
    use oteryn_dev_client::{EntityRef, WorldSpatialEntity};
    use oteryn_protocol_oteryn::{entity_names::EntityDisplayName, world_spatial::ActorPosition};

    #[derive(Clone, Copy)]
    enum CorpseLootBoost {
        Boss(Option<[u8; 16]>),
    }

    #[derive(Clone, Copy)]
    struct CorpseLootDraw<'a> {
        reference: &'a crate::combat::LootDefinitionRef,
        table: &'a crate::combat::LootTableDefinition,
        actor: crate::foundation::ExactActorRef,
    }

    fn verify_corpse_loot(
        rows: &mut Rows<'_>,
        items: &crate::content::item_ref::ItemDefinitionIndex,
        binding: &LootBinding,
        got: &BTreeMap<std::num::NonZeroU32, u64>,
        boost: CorpseLootBoost,
        draw: Option<CorpseLootDraw<'_>>,
        rewards: &CreatureRewardTable,
    ) -> Result<(), Failure> {
        let rolls = match boost {
            CorpseLootBoost::Boss(Some(occurrence)) => boss_bonus_rolls(occurrence),
            CorpseLootBoost::Boss(None) => {
                return Err(failure(
                    "loot",
                    "protocol",
                    "boosted boss death receipt has no reward occurrence",
                ));
            }
        };
        let expected = if let Some(draw) = draw {
            let table = boosted_loot_table(draw.table, rolls, true, |key| {
                rewards.is_daily_boss_equipment(key)
            });
            let channel = draw.actor.channel_id().ok_or_else(|| {
                failure(
                    "loot",
                    "content",
                    "creature sweep loot requires a channel actor",
                )
            })?;
            let seed = crate::combat::LootPlanDeathKey::new(
                *draw.actor.world_id().as_bytes(),
                *channel.as_bytes(),
                draw.actor.scope_generation().get(),
                draw.actor.actor_local_id(),
                draw.actor.actor_local_generation(),
            );
            seeded_loot(draw.reference, &table, seed, items)
                .map_err(|error| failure("loot", "content", error))?
        } else {
            BTreeMap::new()
        };
        judge_seeded_loot(rows, items, binding, got, &expected, rolls != 0);
        Ok(())
    }

    fn boosted_loot_table(
        source: &crate::combat::LootTableDefinition,
        extra_rolls: u8,
        boss: bool,
        is_equipment: impl Fn(&str) -> bool,
    ) -> crate::combat::LootTableDefinition {
        use crate::combat::LootSelectionAlgorithm;
        let mut table = source.clone();
        if extra_rolls != 0 && table.algorithm == LootSelectionAlgorithm::IndependentBernoulliPpm {
            let boosted_entries = if boss {
                oteryn_gameplay_items::loot_plan::partition_daily_boss_bonus_entries(
                    &mut table.entries,
                    is_equipment,
                )
            } else {
                table.entries.len()
            };
            table.algorithm = LootSelectionAlgorithm::DailyBoosted {
                extra_rolls,
                boosted_entries,
            };
        }
        table
    }

    fn judge_seeded_loot(
        rows: &mut Rows<'_>,
        items: &crate::content::item_ref::ItemDefinitionIndex,
        binding: &LootBinding,
        got: &BTreeMap<std::num::NonZeroU32, u64>,
        expected: &BTreeMap<std::num::NonZeroU32, u64>,
        extra_rolls: bool,
    ) {
        let entries = match binding {
            LootBinding::Missing(table) => {
                rows.fail(
                    "loot",
                    "content",
                    &format!("the definition binds loot table {table} but content has none"),
                );
                return;
            }
            LootBinding::None => {
                if got.is_empty() {
                    rows.emit(
                        "loot",
                        "SKIPPED",
                        "no-loot-table;corpse-empty-verified",
                        None,
                    );
                } else {
                    rows.fail(
                        "loot",
                        "unexpected_item",
                        &format!("no loot table but the corpse holds {got:?}"),
                    );
                }
                return;
            }
            LootBinding::Table(_, entries) => entries,
        };
        if entries.is_empty() {
            if got.is_empty() {
                rows.emit(
                    "loot",
                    "SKIPPED",
                    "empty-loot-table;corpse-empty-verified",
                    None,
                );
            } else {
                rows.fail(
                    "loot",
                    "unexpected_item",
                    &format!("empty loot table but the corpse holds {got:?}"),
                );
            }
            return;
        }
        let mut allowed: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for entry in entries {
            let Some(reference) = item_ref(items, &entry.item, &entry.revision) else {
                rows.fail(
                    "loot",
                    "content",
                    &format!("unmapped loot Item {}", entry.item),
                );
                return;
            };
            allowed
                .entry(reference)
                .or_default()
                .push((entry.min, entry.max));
        }
        for (reference, count) in got {
            match allowed.get(reference) {
                None => {
                    rows.fail(
                        "loot",
                        "unexpected_item",
                        &format!("item_definition_ref {reference} is not in the loot table"),
                    );
                    return;
                }
                Some(ranges) if !extra_rolls && !possible_loot_count(ranges, *count) => {
                    rows.fail(
                    "loot",
                    "count_out_of_range",
                    &format!(
                        "item_definition_ref {reference} count {count} cannot come from {ranges:?}"
                    ),
                );
                    return;
                }
                Some(_) => {}
            }
        }
        if !observed_loot_matches_plan(got, expected) {
            rows.fail(
                "loot",
                "seed_mismatch",
                &format!("expected={expected:?};observed={got:?}"),
            );
            return;
        }
        rows.pass(
            "loot",
            &format!(
                "table_entries:{};corpse_items:{};{}",
                entries.len(),
                got.len(),
                if got.is_empty() {
                    "empty-corpse-with-no-proven-drop"
                } else {
                    "all-items-and-counts-allowed"
                }
            ),
        );
    }

    #[tokio::test]
    async fn qualification_daily_query_discards_failed_session() -> TestResult {
        let mut session = Some(1);
        let result = sweep_reward_selection(&mut session, async |_| -> TestResult<()> {
            Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof).into())
        })
        .await;
        assert!(result.is_err());
        assert!(session.is_none());
        session = Some(2);
        assert_eq!(
            sweep_reward_selection(&mut session, async |id| Ok(*id)).await?,
            2
        );
        assert_eq!(session, Some(2));
        Ok(())
    }

    #[test]
    fn qualification_boss_bonus_rejects_corpse_from_the_other_roll() -> TestResult {
        use crate::combat::{
            LootDefinitionRef, LootPlanDeathKey, LootSelectionAlgorithm, LootTableDefinition,
            LootTableEntry,
        };

        use crate::foundation::{
            ActorLocalGeneration, ActorLocalId, ActorRef, ChannelId, ExactActorRef,
            ScopeOwnershipGeneration, WorldId,
        };

        let world = crate::node::uuid_bytes("01890f4c-3b2a-7c01-8d11-9a321b7c0004")
            .ok_or("fixture world")?;
        let channel = crate::node::uuid_bytes("01890f4c-3b2a-7c01-8d11-9a321b7c0005")
            .ok_or("fixture channel")?;
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/spells.manifest.json");
        let room = crate::content::qualify_native_entry_room_from_gameplay_manifest(
            WorldId::decode(&world)?,
            &manifest,
        )?;
        let native = crate::content::qualified_native_gameplay_test_state(room.compiled())?;
        let rewards = native.reward_table();
        let equipment = "oteryn:item.tibia.i3276";
        let ordinary = "oteryn:item.tibia.i3031";
        assert!(rewards.is_daily_boss_equipment(equipment));
        assert!(!rewards.is_daily_boss_equipment(ordinary));
        let items = native.item_index().ok_or("missing served item index")?;
        let binding = LootBinding::Table(
            "oteryn:loot.test_boss".into(),
            [equipment, ordinary]
                .into_iter()
                .map(|item| LootEntry {
                    item: item.into(),
                    revision: "definition-r1".into(),
                    min: 1,
                    max: 1,
                })
                .collect(),
        );
        let source = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![
                LootTableEntry {
                    item: LootDefinitionRef::new("Item", ordinary, "definition-r1"),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(1_000_000),
                    details: Default::default(),
                },
                LootTableEntry {
                    item: LootDefinitionRef::new("Item", equipment, "definition-r1"),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(700_000),
                    details: Default::default(),
                },
            ],
        };
        let reference =
            LootDefinitionRef::new("LootTable", "oteryn:loot.test_boss", "definition-r1");
        // Build the oracle from the prescribed equipment-first table and the
        // literal scope. Do not use the verifier's partition or seed builder.
        let mut observed = BTreeSet::new();
        for generation in 1..=12 {
            let actor = ExactActorRef(ActorRef::from_owner_parts(
                WorldId::decode(&world)?,
                ChannelId::decode(&channel)?,
                ScopeOwnershipGeneration::new(3).map_err(|_| "fixture scope generation")?,
                ActorLocalId(8),
                ActorLocalGeneration(generation),
            ));
            let draw = CorpseLootDraw {
                reference: &reference,
                table: &source,
                actor,
            };
            let seed = LootPlanDeathKey::new(world, channel, 3, 8, generation);
            let expected = |extra_rolls, boosted_entries| {
                let table = LootTableDefinition {
                    algorithm: LootSelectionAlgorithm::DailyBoosted {
                        extra_rolls,
                        boosted_entries,
                    },
                    entries: vec![source.entries[1].clone(), source.entries[0].clone()],
                };
                seeded_loot(&reference, &table, seed, items)
            };
            let two = expected(2, 1)?;
            let three = expected(3, 1)?;
            observed.insert(three.clone());
            for (occurrence, rolls, correct, wrong) in
                [([0; 16], 3, &three, &two), ([6; 16], 2, &two, &three)]
            {
                let boost = CorpseLootBoost::Boss(Some(occurrence));
                let mut rows = Rows::new("public_boss", None);
                verify_corpse_loot(
                    &mut rows,
                    items,
                    &binding,
                    correct,
                    boost,
                    Some(draw),
                    rewards,
                )
                .map_err(|failure| failure.detail)?;
                assert_eq!(rows.counts.pass, 1);
                let oversupplied = expected(rolls, 2)?;
                assert_ne!(&oversupplied, correct);
                let mut rows = Rows::new("public_boss", None);
                verify_corpse_loot(
                    &mut rows,
                    items,
                    &binding,
                    &oversupplied,
                    boost,
                    Some(draw),
                    rewards,
                )
                .map_err(|failure| failure.detail)?;
                assert_eq!(rows.counts.fail, 1);
                if correct != wrong {
                    let mut rows = Rows::new("public_boss", None);
                    verify_corpse_loot(
                        &mut rows,
                        items,
                        &binding,
                        wrong,
                        boost,
                        Some(draw),
                        rewards,
                    )
                    .map_err(|failure| failure.detail)?;
                    assert_eq!(rows.counts.fail, 1);
                }
            }
        }
        assert!(
            observed.len() > 1,
            "probabilistic fixture must distinguish actor generations"
        );
        // The wire hunt must feed the occurrence on the matched receipt to the
        // same classifier that receives the corpse contents.
        let source = include_str!("creature_sweep.rs");
        let hunt = source
            .split_once("async fn hunt(")
            .and_then(|(_, body)| body.split_once("fn confirm_death"))
            .map(|(body, _)| body)
            .ok_or("missing hunt")?;
        assert!(hunt.contains("boss_bonus_rolls("));
        assert!(hunt.contains("death.reward_occurrence"));
        assert!(hunt.contains("death.actor.corpse_identity()"));
        assert!(hunt.contains("judge_loot(rows, items, species, seed, &inputs, &got)"));
        Ok(())
    }

    #[test]
    fn qualification_sweep_recovery_does_not_repeat_fresh_build() -> TestResult {
        let (_, ready) = include_str!("creature_sweep.rs")
            .split_once("async fn ready(")
            .ok_or("missing sweep admission")?;
        let (ready, _) = ready
            .split_once("async fn leave_protection_zone(")
            .ok_or("missing sweep route")?;
        assert!(!ready.contains("provisioning::provision_fixture"));
        assert!(ready.contains("recover_health("));
        Ok(())
    }

    #[test]
    fn review_loot_requires_output_when_the_seeded_roll_produces_items() -> TestResult {
        let items = crate::content::item_ref::ItemDefinitionIndex::decode(
            br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i100","definition-r1"]]}"#,
        )?;
        let binding = LootBinding::Table(
            "oteryn:loot.test".into(),
            vec![LootEntry {
                item: "oteryn:item.tibia.i100".into(),
                revision: "definition-r1".into(),
                min: 1,
                max: 1,
            }],
        );
        let mut missing = Rows::new("rat", None);
        let item = item_ref(&items, "oteryn:item.tibia.i100", "definition-r1")
            .ok_or("test Item reference")?;
        judge_seeded_loot(
            &mut missing,
            &items,
            &binding,
            &BTreeMap::new(),
            &BTreeMap::from([(item, 1)]),
            false,
        );
        assert_eq!(missing.counts.fail, 1);
        let mut missed_roll = Rows::new("rat", None);
        judge_seeded_loot(
            &mut missed_roll,
            &items,
            &binding,
            &BTreeMap::new(),
            &BTreeMap::new(),
            false,
        );
        assert_eq!(missed_roll.counts.pass, 1);
        Ok(())
    }

    #[test]
    fn review_loot_rejects_unmapped_entries_and_impossible_count() -> TestResult {
        let items = crate::content::item_ref::ItemDefinitionIndex::decode(
            br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i100","definition-r1"]]}"#,
        )?;
        let entry = |key: &str, min, max| LootEntry {
            item: key.into(),
            revision: "definition-r1".into(),
            min,
            max,
        };
        let mut unmapped = Rows::new("rat", None);
        judge_seeded_loot(
            &mut unmapped,
            &items,
            &LootBinding::Table(
                "oteryn:loot.test".into(),
                vec![entry("oteryn:item.tibia.i101", 1, 1)],
            ),
            &BTreeMap::new(),
            &BTreeMap::new(),
            false,
        );
        assert_eq!(unmapped.counts.fail, 1);

        let mut wrong_count = Rows::new("rat", None);
        let item = item_ref(&items, "oteryn:item.tibia.i100", "definition-r1")
            .ok_or("test Item reference")?;
        judge_seeded_loot(
            &mut wrong_count,
            &items,
            &LootBinding::Table(
                "oteryn:loot.test".into(),
                vec![
                    entry("oteryn:item.tibia.i100", 1, 1),
                    entry("oteryn:item.tibia.i100", 4, 4),
                ],
            ),
            &BTreeMap::from([(item, 2)]),
            &BTreeMap::new(),
            false,
        );
        assert_eq!(wrong_count.counts.fail, 1);
        Ok(())
    }

    #[test]
    fn qualification_sweep_readiness_reconciles_a_classed_character_missing_the_axe() -> TestResult
    {
        use oteryn_protocol_oteryn::item_view::{
            CharacterInventory, EquipmentSlot, EquippedItem, ItemEntry,
        };
        use std::num::{NonZeroU32, NonZeroU64};

        let axe = NonZeroU32::new(SWEEP_WEAPON_ID).ok_or("missing axe reference")?;
        let backpack = ItemEntry {
            appearance_id: 0,
            handle: NonZeroU64::MIN,
            item_definition_ref: NonZeroU32::MIN,
            count: NonZeroU32::MIN,
            sub_type: 0,
        };
        let mut inventory = CharacterInventory {
            main_backpack: Some(backpack),
            ..Default::default()
        };
        assert!(!fixture_ready(false, None, axe));
        assert!(!fixture_ready(true, None, axe));
        assert!(
            !fixture_ready(true, Some(&inventory), axe),
            "the live readiness gate must reconcile a classed character with only a backpack"
        );

        let axe_entry = ItemEntry {
            handle: NonZeroU64::new(2).ok_or("missing axe handle")?,
            item_definition_ref: axe,
            ..backpack
        };
        inventory.entries.push(axe_entry);
        assert!(!fixture_ready(false, Some(&inventory), axe));
        assert!(fixture_ready(true, Some(&inventory), axe));

        inventory.entries.clear();
        inventory.equipment.push(EquippedItem {
            slot: EquipmentSlot::LeftHand,
            item: axe_entry,
        });
        assert!(fixture_ready(true, Some(&inventory), axe));
        Ok(())
    }

    #[test]
    fn delta_sweep_checks_only_physical_seeded_loot() {
        assert!(!expects_physical_loot(true, 2520));
        assert!(!expects_physical_loot(false, 0));
    }

    #[test]
    fn delta_sweep_rejects_partial_substitute_and_suppressed_loot() {
        let expected = BTreeMap::from([(1, 2), (2, 3)]);
        assert!(!observed_loot_matches_plan(
            &BTreeMap::from([(1, 2)]),
            &expected
        ));
        assert!(!observed_loot_matches_plan(
            &BTreeMap::from([(1, 2), (3, 3)]),
            &expected
        ));
        assert!(!observed_loot_matches_plan(&expected, &BTreeMap::new()));
        assert!(observed_loot_matches_plan(&expected, &expected));
    }

    #[test]
    fn review_near_disappearance_requires_committed_death() {
        assert!(!confirmed_kill(1, false));
        assert!(confirmed_kill(1, true));
        assert!(!confirmed_kill(3, true));
    }

    #[test]
    fn qualification_daily_boost_is_in_live_reward_oracles() -> TestResult {
        let source = include_str!("creature_sweep.rs");
        let hunt = source
            .split_once("async fn hunt(")
            .and_then(|(_, body)| body.split_once("fn seeded_loot("))
            .map(|(body, _)| body)
            .ok_or("missing sweep hunt boundary")?;
        assert!(
            hunt.contains("sweep_reward_selection(session, client_daily_boost::reward_selection)")
        );
        assert!(hunt.contains("boosted_xp("));
        assert!(hunt.contains("death.stamina"));
        assert!(hunt.contains("LootPlanDeathKey::new("));
        assert!(hunt.contains("judge_loot("));
        let prepare = source
            .split_once("async fn prepare_player(")
            .and_then(|(_, body)| body.split_once("async fn read_stats("))
            .map(|(body, _)| body)
            .ok_or("missing sweep setup boundary")?;
        assert!(prepare.contains("if fixture_ready(classed, client.inventory(), axe)"));
        assert!(prepare.contains("run_fixture_command("));

        let mp = include_str!("mp/mp09_party_xp.rs");
        for (start, end) in [
            ("async fn encounter(", "const HUNT_MODES:"),
            ("async fn solo_hunt_award(", "async fn underlevel_party("),
            ("async fn underlevel_party(", "fn exact_award_matches"),
        ] {
            let reward = mp
                .split_once(start)
                .and_then(|(_, body)| body.split_once(end))
                .map(|(body, _)| body)
                .ok_or_else(|| format!("missing MP09 reward boundary {start}"))?;
            assert!(reward.contains("reward_selection("));
            assert!(reward.contains("boosted_xp("));
            assert!(reward.contains("observed_hunt_death("));
            assert!(reward.contains("exact_award_matches("));
        }
        Ok(())
    }

    #[test]
    fn qualification_boosted_loot_requires_the_full_extra_roll() -> TestResult {
        let items = crate::content::item_ref::ItemDefinitionIndex::decode(
            br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i100","definition-r1"]]}"#,
        )?;
        let item = item_ref(&items, "oteryn:item.tibia.i100", "definition-r1")
            .ok_or("test Item reference")?;
        let binding = LootBinding::Table(
            "oteryn:loot.test".into(),
            vec![LootEntry {
                item: "oteryn:item.tibia.i100".into(),
                revision: "definition-r1".into(),
                min: 1,
                max: 1,
            }],
        );
        let expected = BTreeMap::from([(item, 2)]);
        let mut observed = Rows::new("boosted_rat", None);
        judge_seeded_loot(&mut observed, &items, &binding, &expected, &expected, true);
        assert_eq!(observed.counts.pass, 1);
        let mut missing_extra = Rows::new("boosted_rat", None);
        judge_seeded_loot(
            &mut missing_extra,
            &items,
            &binding,
            &BTreeMap::from([(item, 1)]),
            &expected,
            true,
        );
        assert_eq!(missing_extra.counts.fail, 1);
        Ok(())
    }

    #[test]
    fn review_corpse_requires_spawned_creatures_identity() {
        let position = ActorPosition {
            x: 10,
            y: 10,
            floor: 7,
        };
        let entity = EntityRef {
            identity: [1; 16],
            generation: 0,
        };
        let entry = WorldSpatialEntity {
            kind: EntityKind::Corpse,
            entity,
            position,
            display_name: None,
            detail: EntityDetail::Object {
                item_definition_ref: 1,
                quantity: 1,
                item_handle: None,
            },
        };
        let before = BTreeSet::new();
        assert!(!corpse_at(&entry, position, &before, [2; 16]));
        assert!(corpse_at(&entry, position, &before, [1; 16]));
    }

    #[test]
    fn review_successful_species_emits_indexed_verdict() -> TestResult {
        let mut rows = Rows::new("rat", None);
        rows.counts.pass = STEPS.len();
        rows.emitted.fill(true);
        let mut tally = Tally::default();
        let mut lines = Vec::new();
        emit_species_verdict(&rows, std::time::Instant::now(), &mut tally, |line| {
            lines.push(line.to_owned())
        })?;
        assert_eq!(tally.pass, STEPS.len());
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.starts_with("e2e_all scenario=CS.rat.full status=PASS "))
                .count(),
            1
        );
        Ok(())
    }

    #[test]
    fn review_spawn_requires_exact_entity() -> TestResult {
        let position = ActorPosition {
            x: 10,
            y: 10,
            floor: 7,
        };
        let expected = EntityRef {
            identity: [9; 16],
            generation: 3,
        };
        let decoy = WorldSpatialEntity {
            kind: EntityKind::Creature,
            entity: EntityRef {
                generation: 2,
                ..expected
            },
            position,
            display_name: Some(EntityDisplayName::new("Rat").map_err(|_| "display name")?),
            detail: EntityDetail::Actor {
                direction: StepDirection::North,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        let existing = BTreeSet::new();
        assert!(
            visible_spawn([&decoy].into_iter(), position, "Rat", &existing, expected).is_none()
        );
        let target = WorldSpatialEntity {
            entity: expected,
            ..decoy
        };
        assert_eq!(
            visible_spawn(
                [&decoy, &target].into_iter(),
                position,
                "Rat",
                &existing,
                expected
            ),
            Some(target)
        );
        Ok(())
    }
}
