//! Creature sweep: every authored creature species is spawned next to the player through the node's
//! qualification `spawn` control command, fought, killed and looted by a real bot over the real wire,
//! with XP, corpse and loot checked against content.
//!
//! Select with `NODE_BOOT_E2E_ONLY=creature_sweep` (never part of the default run).
//! `OTERYN_E2E_CREATURE_BATCH=start:len` selects a slice of the key-sorted species list of
//! `content/creatures/definitions` (default `0:3`). Use `plan` to print the ranges for parallel jobs.
//!
//! Spawn path: `tools/qualification/node_boot/provision.py --spawn` over `NODE_BOOT_CONTROL_SOCKET`
//! (`sudo -n python3`, as `provision.rs` runs the fixture), answered by the node's uid-0 `spawn`
//! control route (`qualification_wild_spawn.rs`, feature `qualification-spawn`).
//!
//! The sweep calibrates on a Cyclops before checking the batch. One shared
//! session carries the classed build between fights. Each species has a
//! 60-second budget including target release and recovery after a player death.
//!
//! Rows per species: `CS.<key>.spawn|kill|xp|corpse|loot|full`. `spawn` carries the node answer, or
//! the rejected token with cause=precondition. A FAIL row makes `run` return Err, which `journeys`
//! propagates with `?`, so the node-boot stage exits non-zero.
//! XP oracle: the authored `experience` through the server's own
//! `crates/gameplay/characters/src/hunting_experience.rs:13 hunting_experience` (called from
//! `crates/gameplay/durability/src/durability/party_kill_xp.rs:369`), with no prey, the observed daily boost, receipt
//! stamina and no premium for a solo kill (`animus_mastery_experience` adds nothing without
//! mastery, `crates/gameplay/combat/src/combat/death_reward.rs:939`).
//! Loot oracle: the authored loot table. `crates/gameplay/items/src/loot_plan.rs:370 roll_entries`
//! draws each entry independently with a 95..105% chance factor (`loot_factor_percent`), so an empty
//! corpse is possible for any table and every item count lies in its entry's min..max.
use super::*;
use crate::content::creature_reward::CreatureRewardTable;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, client_ui::cyclopedia_character as panel,
};
use oteryn_protocol_oteryn::{
    attack, cyclopedia_character as character, logout::LogoutResult, target_move,
};
use std::collections::BTreeSet;

const BATCH_VARIABLE: &str = "OTERYN_E2E_CREATURE_BATCH";
const BATCH_DEFAULT: (usize, usize) = (0, 3);
const STEPS: [&str; 5] = ["spawn", "kill", "xp", "corpse", "loot"];
const SWEEP_LEVEL: u32 = 100;
const SWEEP_CHARACTER: usize = 0;
const VISIBLE_SECONDS: u64 = 30;
const SPECIES_SECONDS: u64 = 60;
const CALIBRATION_SECONDS: u64 = 10;
const CLEANUP_SECONDS: u64 = 5;
const RECOVERY_SECONDS: u64 = 15;
const LOGOUT_SECONDS: u64 = 930;
const JOB_SECONDS: u64 = 45 * 60;
const JOB_OVERHEAD_SECONDS: u64 = 90;
const BATCH_SIZE: usize =
    ((JOB_SECONDS - LOGOUT_SECONDS - JOB_OVERHEAD_SECONDS) / SPECIES_SECONDS) as usize;
const CYCLOPS: &str = "oteryn:creature.cyclops";

fn split_batches<T>(sorted: &[T]) -> impl Iterator<Item = (usize, &[T])> {
    sorted
        .chunks(BATCH_SIZE)
        .enumerate()
        .map(|(index, chunk)| (index * BATCH_SIZE, chunk))
}

fn select_batch<T>(sorted: &[T], start: usize, len: usize) -> TestResult<&[T]> {
    if start >= sorted.len() {
        return Err(format!(
            "{BATCH_VARIABLE} start {start} is beyond the {} species",
            sorted.len()
        )
        .into());
    }
    let selected = &sorted[start..start.saturating_add(len).min(sorted.len())];
    if selected.len() > BATCH_SIZE {
        return Err(
            format!("{BATCH_VARIABLE} selects more than {BATCH_SIZE} species; use plan").into(),
        );
    }
    Ok(selected)
}

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
    kill: Option<String>,
    combat_seconds: Option<f64>,
    attack_sent: bool,
    calibrating: bool,
}

impl<'a> Rows<'a> {
    fn new(short: &'a str) -> Self {
        Self {
            short,
            emitted: [false; 5],
            lap: std::time::Instant::now(),
            counts: Tally::default(),
            kill: None,
            combat_seconds: None,
            attack_sent: false,
            calibrating: false,
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
        let cause = cause.map_or_else(String::new, |cause| format!(" cause={cause}"));
        evidence(&format!(
            "e2e_all scenario=CS.{}.{step} status={status} seconds={seconds:.3} evidence={}{cause}",
            self.short,
            token(detail)
        ));
    }

    fn pass(&mut self, step: &str, detail: &str) {
        self.emit(step, "PASS", detail, None);
    }

    fn fail(&mut self, step: &str, cause: &str, detail: &str) {
        self.emit(step, "FAIL", detail, Some(cause));
    }

    /// A step that cannot run because an earlier one failed is a FAIL, never a skip.
    fn block_rest(&mut self, root: &str) {
        for (index, step) in STEPS.iter().enumerate() {
            if !self.emitted[index] && (*step != "kill" || self.kill.is_none()) {
                self.emit(step, "FAIL", &format!("blocked-by-{root}"), Some("blocked"));
            }
        }
    }

    fn finish(
        self,
        started: std::time::Instant,
        tally: &mut Tally,
        mut emit: impl FnMut(&str),
    ) -> TestResult {
        let failed = self.counts.fail > 0;
        emit(&format!(
            "creature_sweep species={} seconds={:.3} rows_pass={} rows_fail={} rows_skipped={}",
            self.short,
            started.elapsed().as_secs_f64(),
            self.counts.pass,
            self.counts.fail,
            self.counts.skipped
        ));
        tally.pass += self.counts.pass;
        tally.fail += self.counts.fail;
        tally.skipped += self.counts.skipped;
        if failed {
            return Err(format!("creature {} has FAIL rows", self.short).into());
        }
        emit(&format!(
            "e2e_all scenario=CS.{}.full status=PASS seconds={:.3} evidence=species-checks-completed",
            self.short,
            started.elapsed().as_secs_f64(),
        ));
        Ok(())
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

#[derive(Clone)]
struct Definition {
    name: String,
    experience: u64,
    reward_boss: bool,
}

struct LootEntry {
    item: String,
    min: u64,
    max: u64,
}

enum LootBinding {
    /// The definition binds no loot table: the corpse is minted empty.
    None,
    Table(Vec<LootEntry>),
    Missing(String),
}

struct Species {
    short: String,
    definition: Option<Definition>,
    key: String,
    loot: LootBinding,
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
                    experience: profile["experience"].as_u64().unwrap_or(0),
                    reward_boss: profile["details"]["system_eligibility"]["reward_boss"]
                        .as_bool()
                        .unwrap_or(false),
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
/// the whole map. Yield to the species watchdog during large searches.
async fn far_route(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> Result<Vec<StepDirection>, usize> {
    let mut queue = std::collections::VecDeque::from([from]);
    let mut parents = std::collections::BTreeMap::from([(from, None)]);
    while let Some(at) = queue.pop_front() {
        tokio::task::consume_budget().await;
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
    Err(parents.len())
}

fn item_ref(
    items: &crate::content::item_ref::ItemDefinitionIndex,
    key: &str,
) -> Option<std::num::NonZeroU32> {
    let revision = items.revision(key)?;
    items.definition_ref("Item", key, revision)
}

pub(super) async fn run(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    rewards: &CreatureRewardTable,
) -> TestResult {
    if !std::env::var("NODE_BOOT_E2E_ONLY").is_ok_and(|list| !list.trim().is_empty()) {
        evidence("e2e_groups creature_sweep=skipped reason=opt_in_with_NODE_BOOT_E2E_ONLY");
        return Ok(());
    }
    let batch = match std::env::var(BATCH_VARIABLE) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(format!("invalid {BATCH_VARIABLE}: {error}").into()),
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    verdict::Harness::global().progress("creature-sweep-census");
    let definitions = definitions(&root)?;
    // `definitions` is a BTreeMap, so the species list is sorted by key.
    let keys: Vec<&String> = definitions.keys().collect();
    evidence(&format!(
        "creature_sweep census defined_species={} total={} start={}",
        keys.len(),
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
    if batch.as_deref() == Some("plan") {
        evidence(&format!(
            "creature_sweep plan batches={} size={BATCH_SIZE}",
            split_batches(&keys).count()
        ));
        for (start, chunk) in split_batches(&keys) {
            evidence(&format!(
                "creature_sweep plan batch={start}:{}",
                chunk.len()
            ));
        }
        return Ok(());
    }
    let (start, len) = creature_batch(batch.as_deref())?;
    let batch = select_batch(&keys, start, len)?;
    let tables = loot_tables(&root)?;
    let items = provisioning::item_index()?;
    let mut state = SweepState::default();
    let mut seconds = Vec::new();
    let mut species_failed = 0;
    let setup = async {
        calibrate(
            clients,
            world,
            &items,
            rewards,
            &definitions,
            &mut state.session,
        )
        .await?;
        state.previous_death = death_receipt(clients).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;
    if let Err(error) = setup {
        if let Some(mut client) = state.session.take()
            && let Err(cleanup) = release_bounded(&mut client).await
        {
            report_release_failure(cleanup.as_ref());
        }
        return Err(error);
    }
    evidence(&format!(
        "creature_sweep batch start={start} len={len} selected={}",
        batch.len()
    ));
    for key in batch {
        if state.blocked {
            state.tally.skipped += 1;
            evidence(&format!(
                "e2e_all scenario=CS.{}.full status=SKIPPED seconds=0.000 evidence=block-rest cause=build-unrestorable",
                token(key.strip_prefix("oteryn:creature.").unwrap_or(key))
            ));
            continue;
        }
        let (definition, table) = &definitions[key.as_str()];
        let loot = match table {
            None => LootBinding::None,
            Some(table) => tables.get(table).map_or_else(
                || LootBinding::Missing(table.clone()),
                |entries| {
                    LootBinding::Table(
                        entries
                            .iter()
                            .map(|entry| LootEntry {
                                item: entry.item.clone(),
                                min: entry.min,
                                max: entry.max,
                            })
                            .collect(),
                    )
                },
            ),
        };
        let species = Species {
            short: token(key.strip_prefix("oteryn:creature.").unwrap_or(key)),
            definition: Some(Definition {
                name: definition.name.clone(),
                experience: definition.experience,
                reward_boss: definition.reward_boss,
            }),
            key: (*key).clone(),
            loot,
        };
        let scenario: &'static str =
            Box::leak(format!("CS.{}.full", species.short).into_boxed_str());
        let started = std::time::Instant::now();
        let before = state.tally.pass + state.tally.fail;
        let failures = state.tally.fail;
        verdict::indexed(
            scenario,
            Box::pin(sweep_species(
                clients, world, &items, rewards, &species, &mut state,
            )),
        )
        .await;
        if state.tally.pass + state.tally.fail == before {
            // The outer Watch cancelled the species before it recorded a verdict.
            state.session = None;
            state.tally.fail += 1;
        }
        species_failed += usize::from(state.tally.fail > failures);
        seconds.push(started.elapsed().as_secs_f64());
    }
    if let Some(mut client) = state.session.take()
        && let Err(error) = release_bounded(&mut client).await
    {
        state.tally.fail += 1;
        report_release_failure(error.as_ref());
    }
    let tally = state.tally;
    let mean = seconds.iter().sum::<f64>() / seconds.len().max(1) as f64;
    evidence(&format!(
        "creature_sweep summary species={} pass={} fail={species_failed} skipped={} rows_pass={} rows_fail={} mean_seconds_per_creature={mean:.3}",
        batch.len(),
        seconds.len() - species_failed,
        tally.skipped,
        tally.pass,
        tally.fail
    ));
    if tally.fail > 0 {
        return Err(format!(
            "creature sweep: {} FAIL rows (see the CS.* rows above)",
            tally.fail
        )
        .into());
    }
    Ok(())
}

#[derive(Default)]
struct SweepState {
    session: Option<DevClientSession>,
    tally: Tally,
    previous_death: Option<client_death::DeathReceipt>,
    blocked: bool,
}

fn report_release_failure(error: &dyn std::error::Error) {
    evidence(&format!(
        "creature_sweep release_failed error={}",
        token(&error.to_string())
    ));
}

async fn death_receipt(
    clients: &SeamClients<'_>,
) -> TestResult<Option<client_death::DeathReceipt>> {
    tokio::time::timeout(
        Duration::from_secs(CLEANUP_SECONDS),
        client_death::latest_receipt(clients, SWEEP_CHARACTER),
    )
    .await
    .map_err(|_| "player death receipt deadline")?
}

fn observed_player_death(client: &mut DevClientSession) -> bool {
    let events = client.take_events();
    client.actor_vitals().is_some_and(|vitals| vitals.health == 0)
        || events.into_iter().any(|event| {
            matches!(event, oteryn_dev_client::SessionEvent::ActorVitals(delta) if delta.value.health == 0)
        })
}

async fn clear_target(session: &mut Option<DevClientSession>, rows: &Rows<'_>) -> TestResult {
    if rows.attack_sent
        && let Some(client) = session.as_mut()
    {
        let reply = tokio::time::timeout(
            Duration::from_secs(CLEANUP_SECONDS),
            client.session_mut().attack_target(None),
        )
        .await??;
        if reply.status != CommandStatus::Accepted
            || reply.disposition != attack::AttackIntentDisposition::Ok
        {
            return Err(format!("clear attack target refused: {reply:?}").into());
        }
    }
    Ok(())
}

async fn release_bounded(client: &mut DevClientSession) -> TestResult {
    tokio::time::timeout(Duration::from_secs(LOGOUT_SECONDS), release(client))
        .await
        .map_err(|_| "creature sweep logout deadline")?
}

async fn calibrate(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    rewards: &CreatureRewardTable,
    definitions: &BTreeMap<String, (Definition, Option<String>)>,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    let started = std::time::Instant::now();
    let mut rows = Rows::new("calibration");
    rows.calibrating = true;
    let species = Species {
        short: "calibration".into(),
        definition: definitions
            .get(CYCLOPS)
            .map(|(definition, _)| definition.clone()),
        key: CYCLOPS.into(),
        loot: LootBinding::None,
    };
    let result = tokio::time::timeout(
        Duration::from_secs(SPECIES_SECONDS),
        Box::pin(hunt(
            clients, world, items, rewards, &species, session, &mut rows,
        )),
    )
    .await;
    let cleanup = clear_target(session, &rows).await;
    let died = session.as_mut().is_some_and(observed_player_death);
    let stop = match result {
        Ok(Ok(())) if died => Some(failure("kill", "player-death", "zero health observed")),
        Ok(Ok(())) => cleanup
            .err()
            .map(|error| failure("release", "target-release", error)),
        Ok(Err(stop)) => Some(stop),
        Err(_) => Some(failure("kill", "deadline", "calibration wall-time expired")),
    };
    let seconds = rows
        .combat_seconds
        .unwrap_or_else(|| started.elapsed().as_secs_f64());
    let stop = stop.or_else(|| {
        (!rows
            .combat_seconds
            .is_some_and(|seconds| seconds <= CALIBRATION_SECONDS as f64))
        .then(|| {
            failure(
                "kill",
                "cyclops-dps",
                format!("no confirmed kill within {CALIBRATION_SECONDS}s"),
            )
        })
    });
    let (status, detail) = match &stop {
        Some(stop) => (
            "FAIL",
            format!(" cause={} evidence={}", stop.cause, token(&stop.detail)),
        ),
        None => ("PASS", String::new()),
    };
    evidence(&format!(
        "creature_sweep calibration species=cyclops status={status} seconds={seconds:.3}{detail}"
    ));
    if let Some(stop) = stop {
        return Err(format!(
            "creature_sweep calibration FAIL cause={}: {}",
            stop.cause, stop.detail
        )
        .into());
    }
    Ok(())
}

async fn sweep_species(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    rewards: &CreatureRewardTable,
    species: &Species,
    state: &mut SweepState,
) -> TestResult {
    let started = std::time::Instant::now();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(SPECIES_SECONDS);
    verdict::Harness::global().bounded_wait(deadline, "creature-sweep-species");
    let mut rows = Rows::new(&species.short);
    let result = tokio::time::timeout_at(
        deadline - Duration::from_secs(2 * CLEANUP_SECONDS + RECOVERY_SECONDS),
        Box::pin(hunt(
            clients,
            world,
            items,
            rewards,
            species,
            &mut state.session,
            &mut rows,
        )),
    )
    .await;
    let cleanup = clear_target(&mut state.session, &rows).await;
    let wire_death = state.session.as_mut().is_some_and(observed_player_death);
    let receipt = death_receipt(clients).await;
    let died = wire_death
        || matches!(&result, Ok(Err(stop)) if stop.cause == "player-death")
        || receipt
            .as_ref()
            .is_ok_and(|current| current != &state.previous_death);
    if died {
        rows.kill = None;
        rows.fail("kill", "player-death", "player died during species sweep");
        rows.block_rest("kill");
        let restored = tokio::time::timeout_at(deadline, async {
            let mut receipt = receipt?;
            while receipt == state.previous_death {
                tokio::time::sleep(Duration::from_millis(100)).await;
                receipt = death_receipt(clients).await?;
            }
            state.previous_death = receipt;
            recover_player(clients, world, &mut state.session).await
        })
        .await;
        if !matches!(restored, Ok(Ok(()))) {
            state.blocked = true;
            evidence(&format!(
                "creature_sweep recovery status=FAIL cause=build-unrestorable detail={}",
                token(&format!("{restored:?}"))
            ));
        }
    } else {
        if let Some(detail) = rows.kill.take() {
            rows.pass("kill", &detail);
        }
        let stop = match result {
            Ok(Ok(())) => None,
            Ok(Err(stop)) => Some(stop),
            Err(_) => {
                let step = STEPS
                    .iter()
                    .zip(rows.emitted)
                    .find(|(_, done)| !done)
                    .map_or("loot", |(step, _)| *step);
                Some(failure(
                    step,
                    "deadline",
                    format!("species budget {SPECIES_SECONDS}s"),
                ))
            }
        };
        if let Some(stop) = stop {
            rows.fail(stop.step, stop.cause, &stop.detail);
            rows.block_rest(stop.step);
        }
        if let Err(error) = cleanup {
            state.session = None;
            rows.fail("release", "target-release", &error.to_string());
        }
        if let Err(error) = receipt {
            state.blocked = true;
            rows.fail("recovery", "build-unrestorable", &error.to_string());
        }
    }
    rows.finish(started, &mut state.tally, evidence)
}

/// Let the server settle a temple respawn. If it keeps the old actor dead,
/// leave through ordinary logout and let the next admission consume the respawn.
async fn recover_player(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    let full = |client: &DevClientSession| {
        client.actor_vitals().is_some_and(|vitals| {
            vitals.health == vitals.max_health
                && vitals.mana == vitals.max_mana
                && vitals.health > 0
                && vitals.mana > 0
        })
    };
    if let Some(client) = session.as_mut() {
        let temple = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                client.service_liveness(Duration::from_millis(100)).await?;
                client.take_events();
                if full(client) {
                    break;
                }
            }
            Ok::<(), oteryn_dev_client::DevClientError>(())
        })
        .await;
        match temple {
            Ok(Ok(())) => {}
            Ok(Err(_)) => *session = None,
            Err(_) => {
                let (result, reply, _, _) = logout_retrying_busy(client).await?;
                if result != Some(LogoutResult::Accepted) {
                    return Err(format!("death recovery logout refused: {reply}").into());
                }
                *session = None;
            }
        }
    }
    if session.is_none() {
        *session = Some(connect_character(clients, 0x80, SWEEP_CHARACTER).await?);
    }
    let client = session.as_ref().ok_or("sweep recovery session absent")?;
    if !full(client) {
        return Err("temple or fresh admission did not restore full health and mana".into());
    }
    ready(clients, world, session).await
}

/// The sweep owns a dedicated classed Character and carries its progression between hunts.
async fn ready(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    if session.is_none() {
        *session = Some(connect_character(clients, 0x80, SWEEP_CHARACTER).await?);
    }
    prepare_player(clients, session).await?;
    let client = session.as_mut().ok_or("session absent")?;
    restore_fixture_health(client).await?;
    leave_protection_zone(client, world).await
}

async fn restore_fixture_health(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
    let words = "exura infir ico";
    let mut spell_index = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    tokio::time::timeout_at(deadline, async {
        loop {
            let vitals = client
                .actor_vitals()
                .ok_or("sweep recovery vitals absent")?;
            if vitals.health.saturating_mul(2) >= vitals.max_health {
                return Ok(());
            }
            let index = match spell_index {
                Some(index) => index,
                None => {
                    let index = client_spell_journey::spell_index(native_spell_book()?, words)?;
                    spell_index = Some(index);
                    index
                }
            };
            let cast = client.cast_spell(index, SpellTarget::None, false).await?;
            if !(cast.status == CommandStatus::Accepted
                && cast.disposition == SpellCastDisposition::Cast)
                && cast.disposition != SpellCastDisposition::CoolingDown
            {
                return Err(
                    format!("classed sweep recovery spell {words} refused: {cast:?}").into(),
                );
            }
            client.service_liveness(Duration::from_millis(250)).await?;
            client.take_events();
        }
    })
    .await
    .map_err(|_| "classed sweep recovery did not reach half health")?
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
    let path = far_route(world, from, roomy).await.map_err(|reachable| {
        format!(
            "no open ground from {} ({reachable} tiles reachable)",
            at(from)
        )
    })?;
    walk(client, world, &path).await
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
    rewards: &CreatureRewardTable,
    species: &Species,
    session: &mut Option<DevClientSession>,
    rows: &mut Rows<'_>,
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
        return Err(failure("spawn", "precondition", format!("setup:{error}")));
    }
    let selection = sweep_reward_selection(session, client_daily_boost::reward_selection)
        .await
        .map_err(|error| failure("xp", "protocol", error))?;
    let client = session
        .as_mut()
        .ok_or_else(|| failure("spawn", "precondition", "session absent after setup"))?;
    let (xp_before, level, _) = read_stats(client)
        .await
        .map_err(|error| failure("spawn", "precondition", format!("stats:{error}")))?;

    let name = definition.name.as_str();
    // Leftovers of an earlier species never count as this species' spawn.
    let existing: BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| creature_named(entry, name))
        .map(|entry| entry.entity)
        .collect();
    let session_id = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session_id, species.key.clone())
        .await
        .map_err(|reason| failure("spawn", "precondition", format!("rejected:{reason}")))?;
    let expected =
        parse_spawned_entity(&answer).map_err(|error| failure("spawn", "protocol", error))?;
    let visible_deadline = tokio::time::Instant::now() + Duration::from_secs(VISIBLE_SECONDS);
    let target = loop {
        client
            .service_liveness(Duration::from_millis(100))
            .await
            .map_err(|error| failure("spawn", "protocol", error))?;
        verdict::Harness::global().progress("creature-sweep-visible");
        client.take_events();
        let player = client.world_spatial().actor_position;
        let found = client
            .world_entities()
            .and_then(|entities| visible_spawn(entities.iter(), player, name, &existing, expected));
        if let Some(found) = found {
            break found;
        }
        if tokio::time::Instant::now() >= visible_deadline {
            return Err(failure(
                "spawn",
                "absent",
                format!("{answer};no {name} visible within {VISIBLE_SECONDS}s"),
            ));
        }
    };
    let here = tile(client.world_spatial().actor_position)
        .map_err(|error| failure("spawn", "protocol", error))?;
    if !rows.calibrating {
        rows.pass(
            "spawn",
            &format!("answer:{answer};name:{name};player:{}", at(here)),
        );
    }

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
    client.take_events();
    let combat_started = tokio::time::Instant::now();
    rows.attack_sent = true;
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
    let kill_deadline = combat_started + Duration::from_secs(CALIBRATION_SECONDS);
    let gone = loop {
        client
            .service_liveness(Duration::from_millis(50))
            .await
            .map_err(|error| failure("kill", "unkillable", format!("liveness:{error}")))?;
        verdict::Harness::global().progress("creature-sweep-combat");
        if observed_player_death(client) {
            return Err(failure("kill", "player-death", "zero health observed"));
        }
        if rows.calibrating && tokio::time::Instant::now() > kill_deadline {
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
                    damaged |= health_percent < last_health;
                    last_health = health_percent;
                }
            }
        }
    };
    let seconds = combat_started.elapsed().as_secs_f64();
    rows.combat_seconds = Some(seconds);
    let player = client.world_spatial().actor_position;
    let death_distance = last_position
        .x
        .abs_diff(player.x)
        .max(last_position.y.abs_diff(player.y));
    if !gone {
        return Err(failure(
            "kill",
            "cyclops-dps",
            format!(
                "no death within {CALIBRATION_SECONDS}s;damage_observed={damaged};health_percent={last_health};last_position={last_position:?};player={player:?};vitals={:?}",
                client.actor_vitals()
            ),
        ));
    }
    if death_distance > 2 {
        return Err(failure(
            "kill",
            "vanished",
            format!(
                "target left view far from the player;damage_observed={damaged};distance={death_distance};last_position={last_position:?}"
            ),
        ));
    }
    let receipt = committed_hunt_death(
        client,
        clients,
        Some(clients.characters[SWEEP_CHARACTER]),
        &species.key,
        target.entity,
        tokio::time::Instant::now() + Duration::from_secs(20),
    )
    .await;
    if !confirmed_kill(death_distance, receipt.is_ok()) {
        return Err(failure(
            "kill",
            "unconfirmed_death",
            format!(
                "target {:?} disappeared without a matching death: {:?}",
                target.entity,
                receipt.err()
            ),
        ));
    }
    let HuntDeath {
        actor: dead,
        stamina: death_stamina,
        reward_occurrence,
        ..
    } = receipt.map_err(|error| failure("kill", "unconfirmed_death", error))?;
    let corpse_identity = dead.corpse_identity();
    if rows.calibrating {
        return Ok(());
    }
    rows.kill = Some(format!(
        "seconds:{seconds:.1};death_receipt:matched;damage_observed:{damaged};death_tile:{last_position:?}"
    ));

    // Corpse: a new corpse appears at the death tile.
    let corpse_deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let corpse = loop {
        let found = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entry| corpse_at(entry, last_position, &corpses_before, corpse_identity))
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
        client.take_events();
    };

    // XP: the gain equals the server's value for the authored experience.
    let pool = client_daily_boost::boosted_xp(definition.experience, &species.key, &selection)
        .map_err(|error| failure("xp", "content", error))?;
    let expected = expected_xp(pool, death_stamina, level)?;
    let xp_deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    let gained = loop {
        let (xp, _, _) = read_stats(client)
            .await
            .map_err(|error| failure("xp", "protocol", error))?;
        let gained = xp.saturating_sub(xp_before);
        let settled = if expected == 0 {
            tokio::time::Instant::now() >= xp_deadline - Duration::from_secs(17)
        } else {
            gained >= expected
        };
        if settled || tokio::time::Instant::now() >= xp_deadline {
            break gained;
        }
        client
            .service_liveness(Duration::from_millis(250))
            .await
            .map_err(|error| failure("xp", "protocol", error))?;
        verdict::Harness::global().progress("creature-sweep-xp");
        client.take_events();
    };
    let xp_detail = format!(
        "authored:{};expected:{expected};gained:{gained};level:{level}",
        definition.experience
    );
    if gained == expected {
        rows.pass("xp", &xp_detail);
    } else {
        rows.fail("xp", "xp_mismatch", &xp_detail);
    }

    let Some(corpse) = corpse else {
        rows.fail(
            "corpse",
            "missing",
            &format!("no new corpse at the death tile {last_position:?}"),
        );
        rows.block_rest("corpse");
        return Ok(());
    };
    rows.pass(
        "corpse",
        &format!("entity:{:?};at:{last_position:?}", corpse.entity),
    );
    let (EntityDetail::Object {
        item_handle: Some(handle),
        ..
    }
    | EntityDetail::ObjectWithAppearance {
        item_handle: Some(handle),
        ..
    }) = corpse.detail
    else {
        rows.fail(
            "loot",
            "protocol",
            "the corpse has no server-issued item handle",
        );
        return Ok(());
    };

    let creature_boosted = selection.creature_key == species.key;
    let boss_boosted = selection.boss_key == species.key;
    let physical = matches!(&species.loot, LootBinding::Table(entries) if !entries.is_empty())
        && expects_physical_loot(definition.reward_boss, death_stamina);
    let boost = match (physical, boss_boosted, creature_boosted) {
        (true, true, _) => CorpseLootBoost::Boss(reward_occurrence),
        (true, false, true) => CorpseLootBoost::Creature,
        _ => CorpseLootBoost::None,
    };
    match open_corpse(client, handle).await {
        Err(error) => rows.fail("loot", "protocol", &error.to_string()),
        Ok(got) => {
            let row = if physical {
                Some(rewards.row(&species.key).map_err(|error| {
                    failure("loot", "content", format!("pinned loot row: {error:?}"))
                })?)
            } else {
                None
            };
            let draw = row
                .as_ref()
                .map(|row| {
                    let reference = row.loot_table_ref.as_ref().ok_or_else(|| {
                        failure("loot", "content", "pinned loot table has no identity")
                    })?;
                    Ok(CorpseLootDraw {
                        reference,
                        table: &row.loot_table,
                        actor: dead,
                    })
                })
                .transpose()?;
            verify_corpse_loot(rows, items, &species.loot, &got, boost, draw, rewards)?;
        }
    }
    Ok(())
}

fn expected_xp(pool: u64, stamina: u16, level: u32) -> Result<u64, Failure> {
    let expected = oteryn_gameplay_characters::hunting_experience::hunting_experience(
        i64::try_from(pool).map_err(|error| failure("xp", "content", error))?,
        0,
        0,
        i32::from(stamina),
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
    u64::try_from(expected).map_err(|error| failure("xp", "content", error))
}

#[derive(Clone, Copy)]
enum CorpseLootBoost {
    None,
    Creature,
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
        CorpseLootBoost::None => 0,
        CorpseLootBoost::Creature => 1,
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
        let table = boosted_loot_table(
            draw.table,
            rolls,
            matches!(boost, CorpseLootBoost::Boss(_)),
            |key| rewards.is_daily_boss_equipment(key),
        );
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
    judge_loot(rows, items, binding, got, &expected, rolls != 0);
    Ok(())
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

// Each entry can miss, or contribute any count in its own interval. Merge the
// reachable intervals after each entry so gaps are preserved without expanding
// individual stack counts.
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

fn judge_loot(
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
        LootBinding::Table(entries) => entries,
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
        let Some(reference) = item_ref(items, &entry.item) else {
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
    if !matches_seeded_loot(got, expected) {
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

fn fixture_ready(_level: u32, classed: bool, equipped_and_supplied: bool) -> bool {
    classed && equipped_and_supplied
}

fn expects_physical_loot(private_reward: bool, stamina: u16) -> bool {
    !private_reward && oteryn_gameplay_characters::stamina::hunting_loot_allowed(stamina)
}

fn matches_seeded_loot<K: Ord>(got: &BTreeMap<K, u64>, expected: &BTreeMap<K, u64>) -> bool {
    got == expected
}

/// Provision the level-100 axe build once (the fenced fixture command), then reuse it.
async fn prepare_player(
    clients: &SeamClients<'_>,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    let expected = character::vocation_id("knight");
    let items = provisioning::item_index()?;
    let axe = provisioning::item_reference(&items, 3276)?;
    let rope = provisioning::item_reference(&items, 3003)?;
    let shovel = provisioning::item_reference(&items, 3457)?;
    for verify in [false, true] {
        let client = session.as_mut().ok_or("session absent")?;
        let (_, level, vocation) = read_stats(client).await?;
        let classed = vocation.is_some() && vocation == expected;
        let supplied = client.inventory().is_some_and(|inventory| {
            inventory.main_backpack.is_some()
                && inventory
                    .equipment
                    .iter()
                    .any(|slot| slot.item.item_definition_ref == axe && slot.item.count.get() == 1)
                && [rope, shovel].iter().all(|reference| {
                    inventory
                        .entries
                        .iter()
                        .chain(inventory.equipment.iter().map(|slot| &slot.item))
                        .any(|item| item.item_definition_ref == *reference && item.count.get() == 1)
                })
        });
        if fixture_ready(level, classed, supplied) {
            return Ok(());
        }
        if verify {
            break;
        }
        let fresh = session.take().ok_or("session absent")?;
        *session = Some(
            provisioning::provision_fixture(fresh, clients, SWEEP_CHARACTER, SWEEP_LEVEL).await?,
        );
    }
    Err("classed sweep fixture missing equipped axe or supplies".into())
}

/// Experience and level from the Cyclopedia GeneralStats tab (the same panel MP09 reads).
async fn read_stats(client: &mut DevClientSession) -> TestResult<(u64, u32, Option<u8>)> {
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
async fn release(client: &mut DevClientSession) -> TestResult {
    let mut activity_due = tokio::time::Instant::now();
    loop {
        verdict::Harness::global().progress("creature-sweep-logout");
        let (result, reply, attempts, busy_timed_out) = logout_retrying_busy(client).await?;
        match result {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::InFight { .. }) => {
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

#[cfg(test)]
mod evidence_tests {
    use super::*;
    use oteryn_dev_client::{EntityRef, WorldSpatialEntity};
    use oteryn_protocol_oteryn::{entity_names::EntityDisplayName, world_spatial::ActorPosition};

    #[test]
    fn creature_sweep_plan_covers_sorted_species_within_job_budget() -> TestResult {
        let sorted: Vec<_> = (0..1865).collect();
        let batches: Vec<_> = split_batches(&sorted).collect();
        assert_eq!(batches.len(), 67);
        assert_eq!(
            batches.last().map(|(start, chunk)| (*start, chunk.len())),
            Some((1848, 17))
        );
        let mut seen = Vec::new();
        for (start, chunk) in batches {
            assert!(
                chunk.len() as u64 * SPECIES_SECONDS + LOGOUT_SECONDS + JOB_OVERHEAD_SECONDS
                    <= JOB_SECONDS
            );
            assert_eq!(select_batch(&sorted, start, chunk.len())?, chunk);
            seen.extend_from_slice(chunk);
        }
        assert_eq!(seen, sorted);
        assert_eq!(select_batch(&sorted, 1864, usize::MAX)?, &[1864]);
        assert!(select_batch(&sorted, 1865, 1).is_err());
        assert!(select_batch(&sorted, 0, BATCH_SIZE + 1).is_err());
        assert!(creature_batch(Some("0:0")).is_err());
        assert_eq!(creature_batch(Some("28:17"))?, (28, 17));
        Ok(())
    }

    #[test]
    fn creature_sweep_xp_uses_receipt_stamina() {
        assert_eq!(expected_xp(100, 2520, 100).ok(), Some(100));
        assert_eq!(expected_xp(100, 600, 100).ok(), Some(50));
        assert_eq!(expected_xp(100, 0, 100).ok(), Some(0));
        assert!(expected_xp(u64::MAX, 2520, 100).is_err());
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
            [equipment, ordinary]
                .into_iter()
                .map(|item| LootEntry {
                    item: item.into(),
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
                let mut rows = Rows::new("public_boss");
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
                let mut rows = Rows::new("public_boss");
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
                    let mut rows = Rows::new("public_boss");
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
            .and_then(|(_, body)| body.split_once("enum CorpseLootBoost"))
            .map(|(body, _)| body)
            .ok_or("missing hunt")?;
        assert!(hunt.contains("CorpseLootBoost::Boss(reward_occurrence)"));
        assert!(hunt.contains("actor: dead"));
        assert!(hunt.contains("boost, draw, rewards"));
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
        assert!(ready.contains("restore_fixture_health("));
        Ok(())
    }

    #[test]
    fn review_loot_requires_output_when_the_seeded_roll_produces_items() -> TestResult {
        let items = crate::content::item_ref::ItemDefinitionIndex::decode(
            br#"{"schema":"OTERYN_NATIVE_ITEM_KEYS/v1","records":[["oteryn:item.tibia.i100","definition-r1"]]}"#,
        )?;
        let binding = LootBinding::Table(vec![LootEntry {
            item: "oteryn:item.tibia.i100".into(),
            min: 1,
            max: 1,
        }]);
        let mut missing = Rows::new("rat");
        let item = item_ref(&items, "oteryn:item.tibia.i100").ok_or("test Item reference")?;
        judge_loot(
            &mut missing,
            &items,
            &binding,
            &BTreeMap::new(),
            &BTreeMap::from([(item, 1)]),
            false,
        );
        assert_eq!(missing.counts.fail, 1);
        let mut missed_roll = Rows::new("rat");
        judge_loot(
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
            min,
            max,
        };
        let mut unmapped = Rows::new("rat");
        judge_loot(
            &mut unmapped,
            &items,
            &LootBinding::Table(vec![entry("oteryn:item.tibia.i101", 1, 1)]),
            &BTreeMap::new(),
            &BTreeMap::new(),
            false,
        );
        assert_eq!(unmapped.counts.fail, 1);

        let mut wrong_count = Rows::new("rat");
        let item = item_ref(&items, "oteryn:item.tibia.i100").ok_or("test Item reference")?;
        judge_loot(
            &mut wrong_count,
            &items,
            &LootBinding::Table(vec![
                entry("oteryn:item.tibia.i100", 1, 1),
                entry("oteryn:item.tibia.i100", 4, 4),
            ]),
            &BTreeMap::from([(item, 2)]),
            &BTreeMap::new(),
            false,
        );
        assert_eq!(wrong_count.counts.fail, 1);
        Ok(())
    }

    #[test]
    fn delta_sweep_resumes_incomplete_build_and_reuses_equipped_class_after_death() {
        assert!(!fixture_ready(100, false, false));
        assert!(!fixture_ready(100, true, false));
        assert!(fixture_ready(99, true, true));
    }

    #[test]
    fn delta_sweep_checks_only_physical_seeded_loot() {
        assert!(!expects_physical_loot(true, 2520));
        assert!(!expects_physical_loot(false, 0));
    }

    #[test]
    fn delta_sweep_rejects_partial_substitute_and_suppressed_loot() {
        let expected = BTreeMap::from([(1, 2), (2, 3)]);
        assert!(!matches_seeded_loot(&BTreeMap::from([(1, 2)]), &expected));
        assert!(!matches_seeded_loot(
            &BTreeMap::from([(1, 2), (3, 3)]),
            &expected
        ));
        assert!(!matches_seeded_loot(&expected, &BTreeMap::new()));
        assert!(matches_seeded_loot(&expected, &expected));
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
        assert!(hunt.contains("expects_physical_loot(definition.reward_boss, death_stamina)"));
        assert!(hunt.contains("CorpseLootDraw {"));
        assert!(hunt.contains("verify_corpse_loot("));
        let prepare = source
            .split_once("async fn prepare_player(")
            .and_then(|(_, body)| body.split_once("async fn read_stats("))
            .map(|(body, _)| body)
            .ok_or("missing sweep setup boundary")?;
        assert!(prepare.contains("inventory.main_backpack.is_some()"));
        assert!(prepare.contains("fixture_ready(level, classed, supplied)"));
        assert!(prepare.contains("provisioning::provision_fixture("));

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
        let item = item_ref(&items, "oteryn:item.tibia.i100").ok_or("test Item reference")?;
        let binding = LootBinding::Table(vec![LootEntry {
            item: "oteryn:item.tibia.i100".into(),
            min: 1,
            max: 1,
        }]);
        let expected = BTreeMap::from([(item, 2)]);
        let mut observed = Rows::new("boosted_rat");
        judge_loot(&mut observed, &items, &binding, &expected, &expected, true);
        assert_eq!(observed.counts.pass, 1);
        let mut missing_extra = Rows::new("boosted_rat");
        judge_loot(
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
            motion: None,
        };
        let before = BTreeSet::new();
        assert!(!corpse_at(&entry, position, &before, [2; 16]));
        assert!(corpse_at(&entry, position, &before, [1; 16]));
    }

    #[test]
    fn review_successful_species_emits_indexed_verdict() -> TestResult {
        let mut rows = Rows::new("rat");
        rows.counts.pass = STEPS.len();
        rows.emitted.fill(true);
        let mut tally = Tally::default();
        let mut lines = Vec::new();
        rows.finish(std::time::Instant::now(), &mut tally, |line| {
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
            motion: None,
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
