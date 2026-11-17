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
//! The sweep heals between fights and drops the session after a failed species. Each kill requires
//! wire damage, an explicit removal, and a corpse bound to the exact actor generation.
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
use oteryn_dev_client::{DevClientSession, EntityKind, client_ui::cyclopedia_character as panel};
use oteryn_protocol_oteryn::{
    attack, cyclopedia_character as character, logout::LogoutResult, target_move,
};

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
        let cause = cause.map_or_else(String::new, |cause| {
            let root = if cause == "blocked-by" {
                detail.strip_prefix("blocked-by-").unwrap_or(step)
            } else {
                step
            };
            let cause = cause.replace('_', "-");
            format!(" cause={cause} root=CS.{}.{root}", self.short)
        });
        evidence(&format!(
            "e2e_all scenario=CS.{}.{step} status={status} seconds={seconds:.3} evidence={}{cause}",
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
        };
        let scenario: &'static str =
            Box::leak(format!("CS.{}.full", species.short).into_boxed_str());
        let started = std::time::Instant::now();
        let failures_before = tally.fail;
        let passed = verdict::indexed(
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
        if !passed && tally.fail == failures_before {
            tally.fail += 1;
            // Cancellation can interrupt an in-flight frame. The next hunt must reconnect.
            session = None;
        }
        seconds.push(started.elapsed().as_secs_f64());
    }
    if let Some(mut client) = session.take()
        && let Err(error) = release(&mut client).await
    {
        tally.fail += 1;
        evidence(&format!(
            "creature_sweep release_failed error={}",
            token(&error.to_string())
        ));
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
    // The indexed watch holds the final verdict until this journey returns.
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
    if !reuse_hunt_session(!failed) {
        *session = None;
        return Err(format!("creature {} has FAIL rows", species.short).into());
    }
    evidence(&format!(
        "e2e_all scenario=CS.{}.full status=PASS seconds={:.3}",
        species.short,
        started.elapsed().as_secs_f64()
    ));
    Ok(())
}

/// Admit and provision once. Restore health through ordinary healing before the next hunt.
async fn restore_health(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    client_readmission::within_retry_budget(deadline, async {
        loop {
            let vitals = client
                .actor_vitals()
                .ok_or("creature sweep has no vitals")?;
            if vitals.health.saturating_mul(2) >= vitals.max_health {
                return Ok(());
            }
            client_spell_journey::say(client, "exura").await?;
            client.service_liveness(Duration::from_secs(1)).await?;
            client.take_events();
        }
    })
    .await
}

async fn ready(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    session: &mut Option<DevClientSession>,
) -> TestResult {
    if session.is_none() {
        *session = Some(connect_character(clients, 0x80, CORE_CHARACTER).await?);
    }
    prepare_player(clients, session).await?;
    let low = session
        .as_ref()
        .and_then(|client| client.actor_vitals())
        .is_some_and(|vitals| vitals.health.saturating_mul(2) < vitals.max_health);
    if low {
        restore_health(session.as_mut().ok_or("session absent")?).await?;
    }
    let client = session.as_mut().ok_or("session absent")?;
    leave_protection_zone(client, world).await
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
    let path = reach_route(client, world, from, roomy)
        .await?
        .map_err(|failure| format!("no open ground from {} ({failure})", at(from)))?;
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
    let (xp_before, level) = read_stats(client)
        .await
        .map_err(|error| failure("spawn", "precondition", format!("stats:{error}")))?;

    let name = definition.name.as_str();
    let named = |entry: &oteryn_dev_client::WorldSpatialEntity| {
        entry.kind == EntityKind::Creature
            && client_hunt::health(entry).is_some()
            && entry.display_name.is_some_and(|label| {
                label
                    .as_str()
                    .is_ok_and(|shown| shown.eq_ignore_ascii_case(name))
            })
    };
    let session_id = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session_id, species.key.clone())
        .await
        .map_err(|reason| failure("spawn", "precondition", format!("rejected:{reason}")))?;
    let mut fields = answer.split_whitespace();
    let (Some("spawned"), Some(identity), Some(generation), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(failure("spawn", "protocol", "invalid spawn response"));
    };
    let exact = oteryn_dev_client::EntityRef {
        identity: u128::from_str_radix(identity, 16)
            .map_err(|error| failure("spawn", "protocol", error))?
            .to_be_bytes(),
        generation: generation
            .parse()
            .map_err(|error| failure("spawn", "protocol", error))?,
    };
    let visible_deadline = tokio::time::Instant::now() + Duration::from_secs(VISIBLE_SECONDS);
    let target = loop {
        client
            .service_liveness(Duration::from_millis(100))
            .await
            .map_err(|error| failure("spawn", "protocol", error))?;
        client.take_events();
        let found = client
            .world_entities()
            .and_then(|entities| entities.get(&exact))
            .filter(|entry| named(entry))
            .copied();
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
    let combat_started = tokio::time::Instant::now();
    let handle = client_hunt::kill(
        client,
        target,
        clients.url,
        Duration::from_secs(KILL_SECONDS),
    )
    .await
    .map_err(|error| failure("kill", "observation", error))?;
    rows.pass(
        "kill",
        &format!(
            "seconds:{:.1};damage-removal-corpse-correlated",
            combat_started.elapsed().as_secs_f64()
        ),
    );

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

    rows.pass(
        "corpse",
        &format!("handle:{handle};exact-actor-death-bound"),
    );

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
                rows.pass("loot", "no-loot-table;corpse-empty-verified");
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
            rows.pass("loot", "empty-loot-table;corpse-empty-verified");
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
    if !resolved_loot_entries(unmapped) {
        rows.fail(
            "loot",
            "content",
            &format!("{unmapped} authored loot definitions could not be resolved"),
        );
        return;
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
    *session = Some(provisioning::provision_fixture(fresh, clients, CORE_CHARACTER).await?);
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
