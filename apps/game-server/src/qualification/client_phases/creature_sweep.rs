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
//! One provisioned Character serves the whole sweep. It heals through native gameplay and
//! re-admits after transport loss. A creature that cannot be killed within the bound
//! gets `kill FAIL cause=deadline`; the sweep releases the target and moves on.
//!
//! Rows per species: `CS.<key>.spawn|kill|xp|corpse|loot|full`. `spawn` carries the node answer, or
//! the rejected token with cause=precondition. A FAIL row makes `run` return Err, which `journeys`
//! propagates with `?`, so the node-boot stage exits non-zero.
//! XP oracle: the authored `experience` through the server's own
//! `crates/gameplay/characters/src/hunting_experience.rs:13 hunting_experience` (called from
//! `crates/gameplay/durability/src/durability/party_kill_xp.rs:369`), with the sampled stamina
//! and event rate, no prey, no boost and no premium for a solo level-100 kill. The fixture has no
//! Animus Masteries (`crates/gameplay/combat/src/combat/death_reward.rs:939`).
//! Loot oracle: recompute the deterministic plan for the spawned actor from the pinned table
//! and compare that plan with the contents returned by the wire corpse view.
use super::*;
use crate::content::creature_reward::{CreatureRewardRow, CreatureRewardTable, NoSettlementReason};
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, client_ui::cyclopedia_character as panel,
};
use oteryn_gameplay_items::loot_plan::{
    LootPlanDeathKey, apply_scheduled_loot_rate, plan_creature_loot,
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
    Table(String, Vec<LootEntry>),
    Missing(String),
}

struct Species {
    short: String,
    definition: Option<Definition>,
    key: String,
    loot: LootBinding,
    private_reward: bool,
    reward: Result<std::sync::Arc<CreatureRewardRow>, NoSettlementReason>,
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
                        table.clone(),
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
            }),
            key: (*key).clone(),
            loot,
            private_reward: rewards.reward_boss_corpse(key).is_some(),
            reward: rewards.row(key),
        };
        let scenario: &'static str =
            Box::leak(format!("CS.{}.full", species.short).into_boxed_str());
        let started = std::time::Instant::now();
        let mut rows = Rows::new(&species.short);
        let outcome = verdict::indexed_checked(
            scenario,
            Box::pin(sweep_species(
                clients,
                world,
                &items,
                &species,
                &mut session,
                &mut rows,
            )),
        )
        .await;
        if outcome.is_err() {
            rows.block_rest("full");
            if rows.counts.fail == 0 {
                tally.fail += 1; // The full verdict failed after all actions reported.
            }
            // A failed exchange may have consumed an ID or read part of a frame.
            session = None;
        }
        let elapsed = started.elapsed().as_secs_f64();
        evidence(&format!(
            "creature_sweep species={} seconds={elapsed:.3} rows_pass={} rows_fail={} rows_skipped={}",
            species.short, rows.counts.pass, rows.counts.fail, rows.counts.skipped
        ));
        tally.pass += rows.counts.pass;
        tally.fail += rows.counts.fail;
        tally.skipped += rows.counts.skipped;
        seconds.push(elapsed);
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

async fn sweep_species(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    species: &Species,
    session: &mut Option<DevClientSession>,
    rows: &mut Rows<'_>,
) -> TestResult {
    if let Err(stop) = Box::pin(hunt(clients, world, items, species, session, rows)).await {
        rows.fail(stop.step, stop.cause, &stop.detail);
        rows.block_rest(stop.step);
    }
    if rows.counts.fail > 0 {
        return Err(format!("creature {} has FAIL rows", species.short).into());
    }
    Ok(())
}

/// Provision only the pristine starter. Heal a classed Character using native spells.
async fn ready(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    if session.is_none() {
        wait_character_released(clients, CORE_CHARACTER).await?;
        *session = Some(connect_character(clients, 0x80, CORE_CHARACTER).await?);
    }
    let client = session.as_mut().ok_or("session absent")?;
    let (_, level, vocation) = read_stats(client).await?;
    let vocation = vocation.ok_or("sweep GeneralStats vocation missing")?;
    let low = needs_healing(client)?;
    match preparation(level, Some(vocation) == character::vocation_id("none"), low)? {
        Preparation::Provision => {
            let fresh = session.take().ok_or("session absent")?;
            *session = Some(provisioning::provision_fixture(fresh, clients, CORE_CHARACTER).await?);
        }
        Preparation::Heal => recover_health(client, world, vocation).await?,
        Preparation::Hunt => {}
    }
    leave_protection_zone(session.as_mut().ok_or("session absent")?, world).await
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
    let (xp_before, level, _) = read_stats(client)
        .await
        .map_err(|error| failure("spawn", "precondition", format!("stats:{error}")))?;

    let name = definition.name.as_str();
    let named = |entry: &oteryn_dev_client::WorldSpatialEntity| {
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
            entities
                .iter()
                .find(|entry| {
                    entry.entity == answer.entity
                        && entry.position.floor == player.floor
                        && named(entry)
                        && !existing.contains(&entry.entity)
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
                if let EntityDetail::Actor { health_percent, .. }
                | EntityDetail::ActorWithIcons { health_percent, .. } = now.detail
                {
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
    let _ = client.session_mut().attack_target(None).await;

    // Corpse: a new corpse appears at the death tile.
    let corpse_deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let corpse = loop {
        let found = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entry| {
                    entry.kind == EntityKind::Corpse
                        && entry.entity.identity == corpse_identity
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
    if let Err(reason) = confirm_death(expects_corpse, corpse.is_some(), death_committed) {
        rows.fail("kill", "death_unconfirmed", reason);
        rows.fail("corpse", "contract_mismatch", reason);
        rows.block_rest("corpse");
        return Ok(());
    }
    rows.pass(
        "kill",
        &format!(
            "seconds:{:.1};damage_observed:{damaged};death_tile:{last_position:?}",
            combat_started.elapsed().as_secs_f64()
        ),
    );

    // The native reward owner publishes the sampled modifiers when settlement finishes.
    let inputs_deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    let inputs = loop {
        if let Some(inputs) =
            physical_loot_inputs(&death).map_err(|error| failure("xp", "evidence", error))?
        {
            break inputs;
        }
        if tokio::time::Instant::now() >= inputs_deadline {
            return Err(failure(
                "xp",
                "evidence",
                "missing reward inputs for the spawned death",
            ));
        }
        client
            .service_liveness(Duration::from_millis(250))
            .await
            .map_err(|error| failure("xp", "protocol", error))?;
        client.take_events();
        verdict::Harness::global().progress("creature-sweep-reward-inputs");
    };
    let base_xp = oteryn_gameplay_foundation::scheduled_events::scale_gain(
        definition.experience,
        inputs.xp_rate,
    );
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
        rows.pass(
            "corpse",
            "committed death has no physical corpse in the pinned contract",
        );
        rows.pass("loot", "no physical loot container in the pinned contract");
        return Ok(());
    };
    rows.pass(
        "corpse",
        &format!("entity:{:?};at:{last_position:?}", corpse.entity),
    );
    let Some(handle) = corpse_handle(corpse.detail) else {
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
        Ok(got) => {
            let seed = LootPlanDeathKey::new(
                clients.world,
                clients.channel,
                clients.scope_generation,
                answer.actor_local_id,
                answer.entity.generation,
            );
            judge_loot(
                rows,
                items,
                species,
                seed,
                (inputs.stamina, inputs.loot_rate),
                &got,
            );
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

struct RewardInputs {
    stamina: Option<u16>,
    loot_rate: u32,
    xp_rate: u32,
}

fn physical_loot_inputs(death: &str) -> TestResult<Option<RewardInputs>> {
    let Some(line) = primary_node_witness("physical_loot_inputs", &[("death", death)])? else {
        return Ok(None);
    };
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
    Ok(Some(RewardInputs {
        stamina,
        loot_rate: field("loot_rate")?.parse()?,
        xp_rate: field("xp_rate")?.parse()?,
    }))
}

fn judge_loot(
    rows: &mut Rows<'_>,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    species: &Species,
    seed: LootPlanDeathKey,
    inputs: (Option<u16>, u32),
    observed: &BTreeMap<std::num::NonZeroU32, u64>,
) {
    match expected_loot(items, species, seed, inputs) {
        Err(error) => rows.fail("loot", "content", &error.to_string()),
        Ok(expected) if observed_loot_matches_plan(&expected, observed) => {
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

fn expected_loot(
    items: &crate::content::item_ref::ItemDefinitionIndex,
    species: &Species,
    seed: LootPlanDeathKey,
    (stamina, rate): (Option<u16>, u32),
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
    for entry in entries {
        if entry.min > entry.max || item_ref(items, &entry.item).is_none() {
            return Err(format!("invalid or unmapped authored loot item {}", entry.item).into());
        }
    }
    let mut table = row.loot_table.clone();
    configure_physical_loot(&mut table, species.private_reward, stamina, rate);
    let plan = plan_creature_loot(seed, reference, &table)
        .map_err(|error| format!("pinned loot plan {}: {error:?}", species.key))?;
    let mut expected = BTreeMap::new();
    for entry in plan
        .entries
        .into_iter()
        .filter(|entry| entry.parent.is_none())
    {
        let reference = items
            .definition_ref("Item", &entry.item.production_key, &entry.item.revision_ref)
            .ok_or_else(|| format!("unmapped pinned loot item {}", entry.item.production_key))?;
        *expected.entry(reference).or_default() += u64::from(entry.quantity);
    }
    Ok(expected)
}

fn configure_physical_loot(
    table: &mut oteryn_gameplay_items::loot_plan::LootTableDefinition,
    private_reward: bool,
    stamina: Option<u16>,
    rate: u32,
) {
    if private_reward
        || stamina.is_some_and(|minutes| {
            !oteryn_gameplay_characters::stamina::hunting_loot_allowed(minutes)
        })
    {
        table.entries.clear();
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

#[derive(Debug, PartialEq, Eq)]
enum Preparation {
    Provision,
    Heal,
    Hunt,
}

fn preparation(level: u32, unclassed: bool, low: bool) -> TestResult<Preparation> {
    if level == 1 && unclassed {
        Ok(Preparation::Provision)
    } else if level < SWEEP_LEVEL || unclassed {
        Err("sweep requires a classed level-100 fixture".into())
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
        assert_eq!(preparation(100, false, true)?, Preparation::Heal);
        assert_eq!(preparation(100, false, false)?, Preparation::Hunt);
        assert!(preparation(99, false, true).is_err());
        Ok(())
    }
}

fn observed_loot_matches_plan(
    expected: &BTreeMap<std::num::NonZeroU32, u64>,
    observed: &BTreeMap<std::num::NonZeroU32, u64>,
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
            &BTreeMap::new(),
            &BTreeMap::new()
        ));
    }
}
