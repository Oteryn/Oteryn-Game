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
//! Rows per species: `CS.<key>.spawn|kill|xp|corpse|loot|full`. `spawn` carries the node answer, or
//! the rejected token with cause=precondition. A FAIL row makes `run` return Err, which `journeys`
//! propagates with `?`, so the node-boot stage exits non-zero.
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
const STEPS: [&str; 5] = ["spawn", "kill", "xp", "corpse", "loot"];
const SWEEP_LEVEL: u32 = 100;
const CORE_CHARACTER: usize = 0;
/// A full stamina bar in minutes (`composed_admission/vitals.rs:191`); no hunting time passes first.
const FULL_STAMINA_MINUTES: i32 = 2520;
const VISIBLE_SECONDS: u64 = 30;
const KILL_SECONDS: u64 = 150;
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
    attackable: bool,
    speed: u64,
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
) -> Option<std::num::NonZeroU32> {
    let revision = items.revision(key)?;
    items.definition_ref("Item", key, revision)
}

// `world` serves the one walk out of the protection zone; the authored spawn table is not used.
pub(super) async fn run(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    _spawns: &oteryn_world_bundle::spawn::Table,
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
        } else if definition.experience == 0 {
            no_experience += 1;
        } else if definition.speed == 0 {
            immobile += 1;
        } else {
            keys.push(key);
        }
    }
    evidence(&format!(
        "creature_sweep census defined={} huntable={} excluded_not_attackable={not_attackable} excluded_no_experience={no_experience} excluded_immobile={immobile} start={}",
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
    for key in &keys[start..end] {
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
                attackable: definition.attackable,
                speed: definition.speed,
            }),
            key: (*key).clone(),
            loot,
        };
        let scenario: &'static str =
            Box::leak(format!("CS.{}.full", species.short).into_boxed_str());
        let started = std::time::Instant::now();
        verdict::indexed(
            scenario,
            Box::pin(sweep_species(
                clients,
                world,
                &items,
                &species,
                &mut session,
                &mut tally,
            )),
        )
        .await;
        seconds.push(started.elapsed().as_secs_f64());
    }
    if let Some(mut client) = session.take() {
        if let Err(error) = release(&mut client).await {
            tally.fail += 1;
            evidence(&format!(
                "creature_sweep release_failed error={}",
                token(&error.to_string())
            ));
        }
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
    session: &mut Option<DevClientSession>,
    tally: &mut Tally,
) -> TestResult {
    let started = std::time::Instant::now();
    let mut rows = Rows::new(&species.short);
    if let Err(stop) = Box::pin(hunt(clients, world, items, species, session, &mut rows)).await {
        rows.fail(stop.step, stop.cause, &stop.detail);
        rows.block_rest(stop.step);
    }
    let failed = rows.counts.fail > 0;
    // The `.full` verdict line is the indexed Watch's own; this line carries the species counts.
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
    tally.skipped += rows.counts.skipped;
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
    if session.is_none() {
        *session = Some(connect_character(clients, 0x80, CORE_CHARACTER).await?);
    }
    prepare_player(clients, session).await?;
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
        *session = Some(connect_character(clients, 0x80, CORE_CHARACTER).await?);
    }
    let client = session.as_mut().ok_or("session absent")?;
    leave_protection_zone(client, world).await
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

/// `spawn <session> <key>` through `provision.py --spawn`; Err carries the rejected token.
async fn request_spawn(session: String, key: String) -> Result<String, String> {
    let socket = required("NODE_BOOT_CONTROL_SOCKET").map_err(|error| error.to_string())?;
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("sudo")
            .args(["-n", "python3"])
            .arg(script)
            .args(["--socket", &socket, "--session", &session, "--spawn", &key])
            .output()
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(match stderr.split_once("reason=") {
        Some((_, rest)) => rest
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect(),
        None => format!(
            "status={};stderr_tail={}",
            output.status,
            stderr
                .replace('\n', " | ")
                .chars()
                .rev()
                .take(400)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        ),
    })
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
    let client = session
        .as_mut()
        .ok_or_else(|| failure("spawn", "precondition", "session absent after setup"))?;
    let (xp_before, level) = read_stats(client)
        .await
        .map_err(|error| failure("spawn", "precondition", format!("stats:{error}")))?;

    let name = definition.name.as_str();
    let named = |entry: &oteryn_dev_client::WorldSpatialEntity| {
        entry.kind == EntityKind::Creature
            && matches!(entry.detail, EntityDetail::Actor { .. })
            && entry.display_name.is_some_and(|label| {
                label
                    .as_str()
                    .is_ok_and(|shown| shown.eq_ignore_ascii_case(name))
            })
    };
    // Leftovers of an earlier species never count as this species' spawn.
    let existing: BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| named(entry))
        .map(|entry| entry.entity)
        .collect();
    let session_id = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session_id, species.key.clone())
        .await
        .map_err(|reason| failure("spawn", "precondition", format!("rejected:{reason}")))?;
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
            entities
                .iter()
                .filter(|entry| {
                    entry.position.floor == player.floor
                        && named(entry)
                        && !existing.contains(&entry.entity)
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
                "spawn",
                "absent",
                format!("{answer};no {name} visible within {VISIBLE_SECONDS}s"),
            ));
        }
    };
    let here = tile(client.world_spatial().actor_position)
        .map_err(|error| failure("spawn", "protocol", error))?;
    rows.pass(
        "spawn",
        &format!("answer:{answer};name:{name};player:{}", at(here)),
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
            "deadline",
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

/// Provision the level-100 build once (the fenced fixture command), then reuse it. The fixture is
/// one-shot per character: a refusal on a character that already reads level 100 means "already
/// provisioned". The weapon is not equipped: the fixture's axe record has unknown level and
/// vocations, so the server refuses the slot; the sweep fights with what the profile carries.
async fn prepare_player(
    clients: &SeamClients<'_>,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    let client = session.as_mut().ok_or("session absent")?;
    let (_, level) = read_stats(client).await?;
    if level >= SWEEP_LEVEL {
        return Ok(());
    }
    let mut client = session.take().ok_or("session absent")?;
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let profile = std::env::var("NODE_BOOT_PROFILE").unwrap_or_else(|_| "sorcerer".into());
    let game_session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let status = tokio::task::spawn_blocking(move || {
        std::process::Command::new("sudo")
            .args(["-n", "python3"])
            .arg(script)
            .args([
                "--socket",
                &socket,
                "--session",
                &game_session,
                "--profile",
                &profile,
            ])
            .status()
    })
    .await??;
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
    *session = Some(fresh);
    if level < SWEEP_LEVEL {
        return Err(format!(
            "node-boot fenced build provisioning failed (fixture exit ok={}, level {level})",
            status.success()
        )
        .into());
    }
    evidence(&format!(
        "creature_sweep provisioned level={level} fixture_ok={} weapon=none_equipped",
        status.success()
    ));
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
