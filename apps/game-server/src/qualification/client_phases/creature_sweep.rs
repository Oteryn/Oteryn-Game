//! Creature sweep: every authored, walk-reachable creature species is reached, fought, killed and
//! looted by a real bot over the real wire, with XP, corpse and loot checked against content.
//!
//! Select with `NODE_BOOT_E2E_ONLY=creature_sweep` (the sweep moves the core Character far from the
//! town and is never part of the default run). `OTERYN_E2E_CREATURE_BATCH=start:len` selects a slice
//! of the stably ordered walkable species (default `0:3`; anything else is an error).
//!
//! Rows per species: `CS.<key>.reach|kill|xp|corpse|loot|full`. A FAIL row makes `run` return Err,
//! which `journeys` propagates with `?`, so the node-boot stage exits non-zero.
//! XP oracle: the authored `experience` through the server's own
//! `crates/gameplay/characters/src/hunting_experience.rs:13 hunting_experience` (called from
//! `crates/gameplay/durability/src/durability/party_kill_xp.rs:369`), with no prey, no boost, full
//! stamina and no premium for a solo level-100 kill (`animus_mastery_experience` adds nothing without
//! mastery, `crates/gameplay/combat/src/combat/death_reward.rs:939`).
//! Loot oracle: the authored loot table. `crates/gameplay/items/src/loot_plan.rs:370 roll_entries`
//! draws each entry independently with a 95..105% chance factor (`loot_factor_percent`), so an empty
//! corpse is possible for any table and every item count lies in its entry's min..max.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, client_ui::cyclopedia_character as panel,
};
use oteryn_protocol_oteryn::{
    attack, cyclopedia_character as character, logout::LogoutResult, target_move,
};
use std::collections::BTreeSet;

const BATCH_VARIABLE: &str = "OTERYN_E2E_CREATURE_BATCH";
const BATCH_DEFAULT: (usize, usize) = (0, 3);
const STEPS: [&str; 5] = ["reach", "kill", "xp", "corpse", "loot"];
const SWEEP_LEVEL: u32 = 100;
const CORE_CHARACTER: usize = 0;
/// A full stamina bar in minutes (`composed_admission/vitals.rs:191`); no hunting time passes first.
const FULL_STAMINA_MINUTES: i32 = 2520;
const VISIBLE_SECONDS: u64 = 90;
const KILL_SECONDS: u64 = 150;
const WALK_ATTEMPTS: usize = 4;

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
}

impl<'a> Rows<'a> {
    fn new(short: &'a str) -> Self {
        Self {
            short,
            emitted: [false; 5],
            lap: std::time::Instant::now(),
            counts: Tally::default(),
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
            if !self.emitted[index] {
                self.emit(step, "FAIL", &format!("blocked-by-{root}"), Some("blocked"));
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

struct Definition {
    name: String,
    experience: u64,
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
    spawns: Vec<TilePos>,
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

fn standable(reached: &BTreeSet<TilePos>, world: &BundleWorld, at: TilePos) -> bool {
    reached.contains(&at)
        && world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
}

/// A non-PZ reached tile within one tile of the authored spawn point.
fn near_reached(reached: &BTreeSet<TilePos>, world: &BundleWorld, point: TilePos) -> bool {
    (-1_i32..=1).any(|dy| {
        (-1_i32..=1).any(|dx| {
            match (
                u16::try_from(i32::from(point.x) + dx),
                u16::try_from(i32::from(point.y) + dy),
            ) {
                (Ok(x), Ok(y)) => standable(
                    reached,
                    world,
                    TilePos {
                        x,
                        y,
                        floor: point.floor,
                    },
                ),
                _ => false,
            }
        })
    })
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
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    if !std::env::var("NODE_BOOT_E2E_ONLY").is_ok_and(|list| !list.trim().is_empty()) {
        evidence("e2e_groups creature_sweep=skipped reason=opt_in_with_NODE_BOOT_E2E_ONLY");
        return Ok(());
    }
    let (start, len) = batch_from_env()?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut by_species: BTreeMap<String, Vec<TilePos>> = BTreeMap::new();
    for source in &spawns.sources {
        for point in &source.points {
            let Some(creature) = spawns.creatures.get(point.creature as usize) else {
                continue;
            };
            by_species
                .entry(creature.key.to_string())
                .or_default()
                .push(TilePos {
                    x: point.x,
                    y: point.y,
                    floor: source.floor,
                });
        }
    }
    verdict::Harness::global().progress("creature-sweep-census");
    let reached: BTreeSet<TilePos> = far_route(world, TOWN, |_| false)
        .err()
        .ok_or("census search unexpectedly reached its goal")?
        .into_iter()
        .collect();
    let walkable: Vec<(&String, &Vec<TilePos>)> = by_species
        .iter()
        .filter(|(_, points)| {
            points
                .iter()
                .any(|point| near_reached(&reached, world, *point))
        })
        .collect();
    evidence(&format!(
        "creature_sweep census spawned_species={} walkable_species={} start={} reached_tiles={}",
        by_species.len(),
        walkable.len(),
        at(TOWN),
        reached.len()
    ));
    for (index, chunk) in walkable.chunks(25).enumerate() {
        let keys: Vec<&str> = chunk.iter().map(|(key, _)| key.as_str()).collect();
        evidence(&format!(
            "creature_sweep walkable_keys from={} keys={}",
            index * 25,
            keys.join(",")
        ));
    }
    if start >= walkable.len() {
        return Err(format!(
            "{BATCH_VARIABLE} start {start} is beyond the {} walkable species",
            walkable.len()
        )
        .into());
    }
    let end = start.saturating_add(len).min(walkable.len());
    evidence(&format!(
        "creature_sweep batch start={start} len={len} selected={}",
        end - start
    ));
    let definitions = definitions(&root)?;
    let tables = loot_tables(&root)?;
    let items = provisioning::item_index()?;
    let mut tally = Tally::default();
    let mut seconds = Vec::new();
    for (key, points) in &walkable[start..end] {
        let found = definitions.get(key.as_str());
        let loot = match found.and_then(|(_, table)| table.as_ref()) {
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
            definition: found.map(|(definition, _)| Definition {
                name: definition.name.clone(),
                experience: definition.experience,
            }),
            spawns: (*points).clone(),
            loot,
        };
        let scenario: &'static str =
            Box::leak(format!("CS.{}.full", species.short).into_boxed_str());
        let started = std::time::Instant::now();
        verdict::indexed(
            scenario,
            Box::pin(sweep_species(clients, world, &items, &species, &mut tally)),
        )
        .await;
        seconds.push(started.elapsed().as_secs_f64());
    }
    let mean = seconds.iter().sum::<f64>() / seconds.len().max(1) as f64;
    evidence(&format!(
        "creature_sweep summary species={} rows_pass={} rows_fail={} rows_skipped={} mean_seconds_per_creature={mean:.3}",
        seconds.len(),
        tally.pass,
        tally.fail,
        tally.skipped
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

async fn sweep_species(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    species: &Species,
    tally: &mut Tally,
) -> TestResult {
    let started = std::time::Instant::now();
    let mut rows = Rows::new(&species.short);
    // A fresh admission for every species: one dropped session never cascades.
    let mut session: Option<DevClientSession> = None;
    if let Err(stop) = Box::pin(hunt(
        clients,
        world,
        items,
        species,
        &mut session,
        &mut rows,
    ))
    .await
    {
        rows.fail(stop.step, stop.cause, &stop.detail);
        rows.block_rest(stop.step);
    }
    let released = match session.take() {
        Some(mut client) => release(&mut client).await,
        None => Ok(()),
    };
    if let Err(error) = &released {
        evidence(&format!(
            "creature_sweep release_failed key={} error={}",
            species.short,
            token(&error.to_string())
        ));
    }
    let failed = rows.counts.fail > 0 || released.is_err();
    // The `.full` verdict line is the indexed Watch's own; this line carries the species counts.
    evidence(&format!(
        "creature_sweep species={} seconds={:.3} rows_pass={} rows_fail={} rows_skipped={} release={}",
        species.short,
        started.elapsed().as_secs_f64(),
        rows.counts.pass,
        rows.counts.fail,
        rows.counts.skipped,
        if released.is_ok() { "ok" } else { "failed" }
    ));
    tally.pass += rows.counts.pass;
    tally.fail += rows.counts.fail + usize::from(released.is_err());
    tally.skipped += rows.counts.skipped;
    if failed {
        return Err(format!("creature {} has FAIL rows", species.short).into());
    }
    Ok(())
}

async fn hunt(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    species: &Species,
    session: &mut Option<DevClientSession>,
    rows: &mut Rows<'_>,
) -> Result<(), Failure> {
    let definition = species.definition.as_ref().ok_or_else(|| {
        failure(
            "reach",
            "content",
            "no creature definition with a display name in content/creatures/definitions",
        )
    })?;
    *session = Some(
        connect_character(clients, 0x80, CORE_CHARACTER)
            .await
            .map_err(|error| failure("reach", "precondition", format!("admission:{error}")))?,
    );
    prepare_player(clients, session)
        .await
        .map_err(|error| failure("reach", "precondition", format!("provisioning:{error}")))?;
    let client = session
        .as_mut()
        .ok_or_else(|| failure("reach", "precondition", "session absent after provisioning"))?;
    let (xp_before, level) = read_stats(client)
        .await
        .map_err(|error| failure("reach", "precondition", format!("stats:{error}")))?;

    // Reach: walk with real STEP commands to a non-PZ tile within one tile of an authored spawn.
    let goal = |tile_at: TilePos| {
        species.spawns.iter().any(|point| {
            point.floor == tile_at.floor
                && point.x.abs_diff(tile_at.x).max(point.y.abs_diff(tile_at.y)) <= 1
        }) && world
            .base()
            .tile(tile_at.x, tile_at.y, tile_at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    };
    let mut steps = 0_usize;
    let mut arrived = false;
    let mut last_error = String::new();
    for attempt in 1..=WALK_ATTEMPTS {
        verdict::Harness::global().progress("creature-sweep-route");
        let from = tile(client.world_spatial().actor_position)
            .map_err(|error| failure("reach", "unreachable", error))?;
        if goal(from) {
            arrived = true;
            break;
        }
        // Drops are one way: a site that cannot reach the goal first returns to the start.
        let path = match far_route(world, from, goal) {
            Ok(path) => path,
            Err(reachable) => match far_route(world, from, |tile_at| tile_at == TOWN) {
                Ok(back) if !back.is_empty() => back,
                _ => {
                    return Err(failure(
                        "reach",
                        "unreachable",
                        format!(
                            "no route from {} ({} tiles reachable) to any spawn of the species",
                            at(from),
                            reachable.len()
                        ),
                    ));
                }
            },
        };
        match walk(client, world, &path).await {
            Ok(()) => steps += path.len(),
            Err(error) => {
                last_error = format!("attempt {attempt}: {error}");
            }
        }
    }
    let here = tile(client.world_spatial().actor_position)
        .map_err(|error| failure("reach", "unreachable", error))?;
    if !arrived && !goal(here) {
        return Err(failure(
            "reach",
            "unreachable",
            format!(
                "not at a spawn after {WALK_ATTEMPTS} walks from {};{last_error}",
                at(here)
            ),
        ));
    }
    // The species must be visible and attackable at the spawn.
    let name = definition.name.as_str();
    let visible_deadline = tokio::time::Instant::now() + Duration::from_secs(VISIBLE_SECONDS);
    let target = loop {
        client
            .service_liveness(Duration::from_millis(100))
            .await
            .map_err(|error| failure("reach", "unreachable", error))?;
        verdict::Harness::global().progress("creature-sweep-visible");
        client.take_events();
        let player = client.world_spatial().actor_position;
        let found = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .filter(|entry| {
                    entry.kind == EntityKind::Creature
                        && entry.position.floor == player.floor
                        && matches!(entry.detail, EntityDetail::Actor { .. })
                        && entry.display_name.is_some_and(|label| {
                            label
                                .as_str()
                                .is_ok_and(|shown| shown.eq_ignore_ascii_case(name))
                        })
                })
                .min_by_key(|entry| {
                    entry
                        .position
                        .x
                        .abs_diff(player.x)
                        .max(entry.position.y.abs_diff(player.y))
                })
                .copied()
        });
        if let Some(found) = found {
            break found;
        }
        if tokio::time::Instant::now() >= visible_deadline {
            return Err(failure(
                "reach",
                "absent",
                format!(
                    "arrived at {} but no {name} was visible within {VISIBLE_SECONDS}s",
                    at(here)
                ),
            ));
        }
    };
    rows.pass(
        "reach",
        &format!("name:{name};steps:{steps};at:{}", at(here)),
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
        EntityDetail::Actor { health_percent, .. } => health_percent,
        EntityDetail::Object { .. } => 100,
    };
    let mut damaged = false;
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
    let kill_deadline = combat_started + Duration::from_secs(KILL_SECONDS);
    let gone = loop {
        client
            .service_liveness(Duration::from_millis(50))
            .await
            .map_err(|error| failure("kill", "unkillable", format!("liveness:{error}")))?;
        verdict::Harness::global().progress("creature-sweep-combat");
        client.take_events();
        match client
            .world_entities()
            .and_then(|entities| entities.get(&target.entity))
        {
            None => break true,
            Some(now) => {
                last_position = now.position;
                if let EntityDetail::Actor { health_percent, .. } = now.detail {
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
    if !gone {
        let _ = client.session_mut().attack_target(None).await;
        return Err(failure(
            "kill",
            "unkillable",
            format!(
                "no death within {KILL_SECONDS}s;damage_observed={damaged};health_percent={last_health};last_position={last_position:?};player={player:?};vitals={:?}",
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
    rows.pass(
        "kill",
        &format!(
            "seconds:{:.1};damage_observed:{damaged};death_tile:{last_position:?}",
            combat_started.elapsed().as_secs_f64()
        ),
    );
    let _ = client.session_mut().attack_target(None).await;

    // Corpse: a new corpse appears at the death tile.
    let corpse_deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let corpse = loop {
        let found = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entry| {
                    entry.kind == EntityKind::Corpse
                        && entry.position == last_position
                        && !corpses_before.contains(&entry.entity)
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
        client.take_events();
    };

    // XP: the gain equals the server's value for the authored experience.
    let expected = oteryn_gameplay_characters::hunting_experience::hunting_experience(
        i64::try_from(definition.experience).map_err(|error| failure("xp", "content", error))?,
        0,
        0,
        FULL_STAMINA_MINUTES,
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
    let gained = loop {
        let (xp, _) = read_stats(client)
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
    let EntityDetail::Object {
        item_handle: Some(handle),
        ..
    } = corpse.detail
    else {
        rows.fail(
            "loot",
            "protocol",
            "the corpse has no server-issued item handle",
        );
        return Ok(());
    };

    // Loot: open the corpse through the real protocol and compare it with the loot table.
    match open_corpse(client, handle).await {
        Err(error) => rows.fail("loot", "protocol", &error.to_string()),
        Ok(got) => judge_loot(rows, items, &species.loot, &got),
    }
    Ok(())
}

fn judge_loot(
    rows: &mut Rows<'_>,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    binding: &LootBinding,
    got: &BTreeMap<std::num::NonZeroU32, u64>,
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
    // Per item: at least one success (its smallest min) up to every entry succeeding (sum of max).
    let mut allowed: BTreeMap<std::num::NonZeroU32, (u64, u64)> = BTreeMap::new();
    let mut unmapped = 0_usize;
    for entry in entries {
        let Some(reference) = item_ref(items, &entry.item) else {
            unmapped += 1;
            continue;
        };
        let range = allowed.entry(reference).or_insert((entry.min, 0));
        range.0 = range.0.min(entry.min);
        range.1 += entry.max;
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
            Some((low, high)) if count < low || count > high => {
                rows.fail(
                    "loot",
                    "count_out_of_range",
                    &format!(
                        "item_definition_ref {reference} count {count} outside {low}..={high}"
                    ),
                );
                return;
            }
            Some(_) => {}
        }
    }
    rows.pass(
        "loot",
        &format!(
            "table_entries:{};unmapped_entries:{unmapped};corpse_items:{};{}",
            entries.len(),
            got.len(),
            if got.is_empty() {
                "empty-corpse-allowed-by-95pct-chance-factor"
            } else {
                "all-items-in-table-within-min-max"
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

/// Provision the level-100 axe build once (the fenced fixture command), then reuse it.
async fn prepare_player(
    clients: &SeamClients<'_>,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    let client = session.as_mut().ok_or("session absent")?;
    let (_, level) = read_stats(client).await?;
    if level >= SWEEP_LEVEL {
        return Ok(());
    }
    let fresh = session.take().ok_or("session absent")?;
    *session = Some(provisioning::provision(fresh, clients, CORE_CHARACTER).await?);
    Ok(())
}

/// Experience and level from the Cyclopedia GeneralStats tab (the same panel MP09 reads).
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
