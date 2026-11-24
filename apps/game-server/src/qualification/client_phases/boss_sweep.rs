//! Opt-in native encounter journeys. The census keeps separate authored records even when
//! multiple catalogue entries name the same creature. A source-only record is a coverage gap.
//! Lever admission is owned by gameplay_transport/boss_activity.rs:175–292, and cooldown
//! publication by durability/activity_handoff/boss_room.rs:585–608.
//! XP uses gameplay/characters/src/hunting_experience.rs:13 with the existing
//! free-account qualification fixture. Qualification never seeds a boss or its rewards.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, client_runtime::boss_encounter as panel,
};
use oteryn_gameplay_encounters::bosses::endgame_entry::{Encounter, QuestFamily};
use oteryn_protocol_oteryn::{
    activity_scope::ActivityScope,
    attack,
    boss_encounter::{self as wire, BossActivityRefusal, BossActivityReply},
};
use serde::Deserialize;

const BATCH_VARIABLE: &str = "OTERYN_E2E_BOSS_SWEEP_BATCH";
const COMBAT_FIXTURE_MINIMUM_LEVEL: u32 = 400;
const STEPS: [&str; 7] = [
    "ineligible",
    "eligible",
    "spawn",
    "kill",
    "reward",
    "cooldown",
    "repeat",
];
const LEVERS: &str = "content/encounters/bosses/crystal-lever-candidates.json";
const SELECTED: &str = "content/encounters/bosses/selected-first-dragon.json";
const ARENAS: &str = "content/encounters/arenas/pve-catalog.json";
const HELD: &str = "content/world/spawns/held-bosses.json";
const ROOM_OWNER: &str = "crates/gameplay/dispatch/src/gameplay_transport/boss_activity.rs:188";
const COOLDOWN_OWNER: &str =
    "crates/gameplay/durability/src/durability/activity_handoff/boss_room.rs:585";

fn batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(value) = value else {
        return Ok((0, 3));
    };
    let number = |part: &str| {
        (!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<usize>().ok())
            .flatten()
    };
    value
        .split_once(':')
        .and_then(|(start, len)| Some((number(start)?, number(len)?)))
        .filter(|&(_, len)| len > 0)
        .ok_or_else(|| format!("invalid {BATCH_VARIABLE}: expected start:len with len > 0").into())
}

#[derive(Deserialize)]
struct Lever {
    path: String,
    line: u32,
    creature_key: Option<String>,
    lever_positions: Vec<[i32; 3]>,
    entry_positions: Vec<[i32; 3]>,
    boss_position: Option<[i32; 3]>,
    minimum_players: Option<usize>,
    minimum_level: Option<u32>,
    cooldown_seconds: Option<u32>,
    after_kill_seconds: Option<u32>,
}

struct HeldSpawn {
    at: TilePos,
    respawn: Duration,
}

struct Entry {
    key: String,
    source: String,
    creature: Option<String>,
    lever: Option<usize>,
    gap: &'static str,
    held: Option<HeldSpawn>,
}

impl Entry {
    fn koshei_switch(&self) -> bool {
        self.creature.as_deref() == Some("oteryn:creature.koshei_the_deathless")
            && self
                .held
                .as_ref()
                .is_some_and(|held| held.at == KOSHEI_HELD)
    }
}

struct Catalogue {
    entries: Vec<Entry>,
    levers: Vec<Lever>,
}

fn required_key(value: &serde_json::Value) -> TestResult<&str> {
    value.as_str().ok_or_else(|| "missing content key".into())
}

fn census(root: &Path) -> TestResult<Catalogue> {
    let read = |name: &str| -> TestResult<serde_json::Value> {
        Ok(serde_json::from_slice(&std::fs::read(root.join(name))?)?)
    };
    let mut levers: Vec<Lever> = serde_json::from_value(read(LEVERS)?["records"].take())?;
    levers.push(serde_json::from_value(read(SELECTED)?["candidate"].take())?);
    let mut entries = Vec::new();
    for (index, lever) in levers.iter().enumerate() {
        let stem = lever.creature_key.as_deref().map_or("unresolved", short);
        let source = if lever.path == SELECTED {
            SELECTED
        } else {
            LEVERS
        };
        // The source locator distinguishes multiple authored levers for the same creature.
        let locator = creature_sweep::token(&lever.path.trim_end_matches(".lua").replace('/', "."));
        entries.push(Entry {
            key: format!("{stem}.lever.{locator}.{}", lever.line),
            source: format!("{source}:1"),
            creature: lever.creature_key.clone(),
            lever: Some(index),
            gap: "",
            held: None,
        });
    }
    let directory = root.join("content/encounters/definitions");
    for file in std::fs::read_dir(&directory)? {
        let file = file?;
        let path = file.path();
        if !file
            .file_name()
            .to_string_lossy()
            .starts_with("encounters-")
        {
            continue;
        }
        let source = path.strip_prefix(root)?.display().to_string();
        let records: Vec<serde_json::Value> =
            serde_json::from_value(read(&source)?["records"].take())?;
        for record in records {
            let key = required_key(&record["declaration"]["identity"]["key"])?;
            let details = &record["authoring"]["profile"]["details"];
            let mut creatures = details["participants"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|participant| participant["creatures"].as_array().into_iter().flatten())
                .filter_map(|creature| creature["key"].as_str());
            let primary = creatures.clone().next();
            // Participant order also includes supporting actors. Match the
            // encounter's own identity anywhere in the authored roster.
            let matching = creatures.find(|creature| short(creature) == short(key));
            let parent =
                room_actor(short(key)).map(|(parent, _)| format!("oteryn:creature.{parent}"));
            let (lever, donor) = donor_lever(&levers, short(key))
                .unwrap_or_else(|| resolve_lever(&levers, parent.as_deref().or(matching)));
            let creature = lever
                .and_then(|index| levers[index].creature_key.as_deref())
                .or(primary);
            let (gap, gap_source) =
                donor_gap(short(key)).unwrap_or(("encounter-has-no-unambiguous-lever-binding", ""));
            let donor = donor.or((!gap_source.is_empty()).then_some(gap_source));
            entries.push(Entry {
                key: format!("{}.encounter", short(key)),
                source: donor.map_or_else(|| format!("{source}:1"), str::to_owned),
                creature: creature.map(str::to_owned),
                lever,
                gap,
                held: None,
            });
        }
    }
    let arenas: Vec<serde_json::Value> =
        serde_json::from_value(read(ARENAS)?["definitions"].take())?;
    for arena in arenas {
        let creature = required_key(&arena["key"])?;
        entries.push(Entry {
            key: format!("{}.arena", short(creature)),
            source: format!("{ARENAS}:1"),
            creature: Some(creature.to_owned()),
            lever: None,
            gap: "not reachable in game;owner:docs/architecture/reviews/OTERYN_GAME_ARENA1_DECISION_2026-10-15.md:19",
            held: None,
        });
    }
    let held: Vec<serde_json::Value> = serde_json::from_value(read(HELD)?["records"].take())?;
    for record in held {
        let point = &record["point"];
        let creature = required_key(&point["creature"])?;
        let [x, y, floor]: [i32; 3] = ["x", "y", "floor"]
            .map(|axis| {
                point["cell"][axis]
                    .as_i64()
                    .and_then(|value| i32::try_from(value).ok())
            })
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or("held spawn has no cell")?
            .try_into()
            .map_err(|_| "invalid held spawn cell")?;
        entries.push(Entry {
            key: format!("{}.held.{x}_{y}_{floor}", short(creature)),
            source: format!("{HELD}:1"),
            creature: Some(creature.to_owned()),
            lever: None,
            gap: "",
            held: Some(HeldSpawn {
                at: cell([x, y, floor])?,
                respawn: Duration::from_millis(
                    point["respawn_ms"]
                        .as_u64()
                        .ok_or("held spawn missing respawn")?,
                ),
            }),
        });
    }
    entries.sort_unstable_by(|left, right| left.key.cmp(&right.key));
    if entries.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err("duplicate boss sweep entry key".into());
    }
    Ok(Catalogue { entries, levers })
}

// These actors are created inside the parent's room by the cited Crystal hooks.
fn room_actor(key: &str) -> Option<(&'static str, &'static str)> {
    Some(match key {
        "charged_anomaly" => ("anomaly", "Charged Anomaly"),
        "charging_outburst" => ("outburst", "Charging Outburst"),
        "crystal_smelly_cheese" => ("ratmiral_blackwhiskers", "Smelly Cheese"),
        "crystal_weak_spot" => ("ratmiral_blackwhiskers", "Weak Spot"),
        "field_fill_aspect_of_power" => ("goshnar_s_megalomania_purple", "Aspect of Power"),
        "field_fill_soul_cage" => ("goshnar_s_malice", "Soul Cage"),
        _ => return None,
    })
}

struct ScriptedRoom {
    unique: Option<u16>,
    action: Option<u16>,
    roster: &'static [[i32; 3]],
    actor: &'static str,
    spawn: Option<[i32; 3]>,
    step_landing: Option<[i32; 3]>,
}

fn scripted_room(key: &str) -> Option<ScriptedRoom> {
    Some(match key {
        "azerus" => ScriptedRoom {
            unique: Some(3086),
            action: None,
            roster: &[],
            actor: "Azerus",
            spawn: None,
            step_landing: None,
        },
        "death_priest_shargon" => ScriptedRoom {
            unique: Some(30002),
            action: None,
            roster: &[
                [33583, 31844, 10],
                [33584, 31844, 10],
                [33585, 31844, 10],
                [33586, 31844, 10],
                [33587, 31844, 10],
            ],
            actor: "Death Priest Shargon",
            spawn: Some([33487, 32108, 9]),
            step_landing: None,
        },
        "drume" => ScriptedRoom {
            unique: None,
            action: Some(59601),
            roster: &[[32457, 32508, 6]],
            actor: "Usurper Commander",
            spawn: Some([32444, 32516, 7]),
            step_landing: None,
        },
        "ferumbras_mortal_shell" => ScriptedRoom {
            unique: None,
            action: Some(24846),
            roster: &[],
            actor: "Ferumbras Mortal Shell",
            spawn: None,
            step_landing: Some([33270, 31474, 14]),
        },
        _ => return None,
    })
}

fn donor_gap(key: &str) -> Option<(&'static str, &'static str)> {
    Some(match key {
        "azerus" => (
            "scripted room",
            "Crystal201189f8:data-global/scripts/quests/in_service_of_yalahar/actions_last_fight.lua:41",
        ),
        "carnisylvan_sapling" => (
            "self-removing combat actor",
            "Crystal201189f8:data-global/scripts/spells/monster/sapling_explode.lua:21",
        ),
        "charged_anomaly" => (
            "phase of Anomaly",
            "Crystal201189f8:data-global/scripts/quests/heart_of_destruction/creaturescripts_anomaly_transform.lua:6",
        ),
        "charging_outburst" => (
            "phase of Outburst",
            "Crystal201189f8:data-global/scripts/quests/heart_of_destruction/creaturescripts_outburst_charge.lua:6",
        ),
        "cobra_bastion" => (
            "zone entry movement; encounter covers ambient Cobra populations",
            "Crystal201189f8:data-global/scripts/quests/grave_danger/cobra_bastion/movements_access_storage.lua:16",
        ),
        "corrupted_soul" => (
            "summoned by Soulcatcher",
            "Crystal201189f8:data-global/scripts/quests/forgotten_knowledge/creaturescripts_soulcatcher_summon.lua:7",
        ),
        "crystal_smelly_cheese" => (
            "Ratmiral phase actor",
            "Crystal201189f8:data-global/scripts/quests/a_pirates_tail_quest/actions_ratmiral_blackwhiskers.lua:7",
        ),
        "crystal_weak_spot" => (
            "Ratmiral phase actor",
            "Crystal201189f8:data-global/scripts/quests/a_pirates_tail_quest/actions_ratmiral_blackwhiskers.lua:32",
        ),
        "cult_soul_remains" => (
            "cultist death trigger",
            "Crystal201189f8:data-global/scripts/quests/cults_of_tibia/creaturescripts_carlin_vortex_spawn.lua:4",
        ),
        "dangerous_depth_makeshift_homes" => (
            "Lost Exile task death trigger",
            "Crystal201189f8:data-global/scripts/quests/dangerous_depth/creaturescripts_lost_exile_task.lua:13",
        ),
        "death_priest_shargon" => (
            "scripted lever absent from compiled lever census",
            "Crystal201189f8:data-global/scripts/quests/dark_trails/actions_lever_death_priest_shargon.lua:60",
        ),
        "deep_terror" => (
            "tentacle-triggered encounter",
            "Crystal201189f8:data-global/scripts/quests/hero_of_rathleton/creaturescripts_tentacle.lua:13",
        ),
        "dragon_egg" => (
            "Frozen Horror room actor",
            "Crystal201189f8:data-global/scripts/quests/forgotten_knowledge/actions_frozen_horror.lua:10",
        ),
        "drume" => (
            "scripted lever absent from compiled lever census",
            "Crystal201189f8:data-global/scripts/quests/the_order_of_lion_quest/action_drume.lua:114",
        ),
        "eruption_of_destruction" => (
            "combat script found, entry trigger unidentified",
            "Crystal201189f8:data-crystal/scripts/spells/monster/eruption_of_destruction_explosion.lua:90",
        ),
        "essence_of_malice" => (
            "scripted lever entry",
            "Crystal201189f8:data-global/scripts/quests/cults_of_tibia/actions_bosses_levers.lua:278",
        ),
        "feroxa" => (
            "Grimvale silvered trap trigger",
            "Crystal201189f8:data-global/scripts/quests/grimvale/movements_silvered_trap.lua:9",
        ),
        "ferumbras_mortal_shell" => (
            "seal movement trigger",
            "Crystal201189f8:data-global/scripts/quests/ferumbras_ascension/movements_seal.lua:47",
        ),
        "field_fill_aspect_of_power" => (
            "Soul War phase actor",
            "Crystal201189f8:data-global/scripts/quests/soul_war/soul_war_mechanics.lua:929",
        ),
        "field_fill_bone_capsule" => (
            "Ferumbras Ascension phase actor",
            "Crystal201189f8:data-global/scripts/quests/ferumbras_ascension/creaturescripts_bone_capsule.lua:5",
        ),
        "field_fill_cerebellum" => (
            "brain room portal actor",
            "Crystal201189f8:data-global/scripts/quests/feaster_of_souls/actions_portal_brain_head.lua:74",
        ),
        "field_fill_destabilized_ferumbras" => (
            "vortex movement trigger",
            "Crystal201189f8:data-global/scripts/quests/ferumbras_ascension/movements_vortex.lua:15",
        ),
        "field_fill_foam_stalker" => (
            "combat spell trigger",
            "Crystal201189f8:data-global/scripts/spells/monster/foam_splash.lua:52",
        ),
        "field_fill_lisa" => (
            "combat script found, entry trigger unidentified",
            "Crystal201189f8:data-crystal/scripts/spells/monster/lisa_heal.lua:23",
        ),
        "field_fill_soul_cage" => (
            "Soul War phase actor",
            "Crystal201189f8:data-global/lib/quests/soul_war.lua:998",
        ),
        "field_fill_tyrn" => (
            "combat script found, entry trigger unidentified",
            "Crystal201189f8:data-crystal/scripts/spells/monster/tyrn_heal.lua:19",
        ),
        "fury_of_the_emperor" => (
            "quest sceptre trigger",
            "Crystal201189f8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission10_a_message_of_freedom_sceptre.lua:2",
        ),
        "gaz_haragoth" => (
            "combat script found, entry trigger unidentified",
            "Crystal201189f8:data-global/scripts/creaturescripts/monster/gaz_haragoth_heal.lua:12",
        ),
        "glooth_generator" => (
            "combat script found, entry trigger unidentified",
            "Crystal201189f8:data-crystal/scripts/spells/monster/generator.lua:13",
        ),
        "glooth_horror" => (
            "scripted spawn",
            "Crystal201189f8:data-global/scripts/quests/hero_of_rathleton/creaturescripts_glooth_horror.lua:4",
        ),
        _ => return None,
    })
}

fn donor_lever(levers: &[Lever], key: &str) -> Option<(Option<usize>, Option<&'static str>)> {
    let (path, creature, source) = match key {
        "crystal_herald_of_fire" => (
            "data-global/scripts/quests/targuna/actions_herald_lever.lua",
            "oteryn:creature.herald_of_fire",
            "Crystal201189f8:data-global/scripts/quests/targuna/actions_herald_lever.lua:6",
        ),
        "crystal_lord_retro" => (
            "data-global/scripts/quests/25_years_of _tibia_quest/lever_lord_retro.lua",
            "oteryn:creature.lord_retro",
            "Crystal201189f8:data-global/scripts/quests/25_years_of _tibia_quest/lever_lord_retro.lua:3",
        ),
        "field_fill_goshnar_s_malice" => (
            "data-global/lib/quests/soul_war.lua",
            "oteryn:creature.goshnar_s_malice",
            "Crystal201189f8:data-global/lib/quests/soul_war.lua:867",
        ),
        "ghulosh" => (
            "data-global/scripts/actions/bosses_levers/ghulosh.lua",
            "oteryn:creature.ghulosh",
            "Crystal201189f8:data-global/scripts/actions/bosses_levers/ghulosh.lua:5",
        ),
        _ => return None,
    };
    let index = levers
        .iter()
        .position(|lever| lever.path == path && lever.creature_key.as_deref() == Some(creature))?;
    Some((Some(index), Some(source)))
}

fn resolve_lever(
    levers: &[Lever],
    creature: Option<&str>,
) -> (Option<usize>, Option<&'static str>) {
    let Some(creature) = creature else {
        return (None, None);
    };
    // Crystal 201189f8 registers the Dreamscar action by position. Its weekday
    // table names Alptramun explicitly and the captured layout has no fixed creature.
    if creature == "oteryn:creature.alptramun" {
        let index = levers.iter().position(|lever| {
            lever.path == "data-global/scripts/quests/the_dream_courts/actions_dreamscar_levers.lua"
                && lever.creature_key.is_none()
                && lever.lever_positions.first() == Some(&[32208, 32021, 13])
        });
        if index.is_some() {
            return (
                index,
                Some(
                    "Crystal201189f8:data-global/scripts/quests/the_dream_courts/actions_dreamscar_levers.lua:74",
                ),
            );
        }
    }
    let mut matching = levers
        .iter()
        .enumerate()
        .filter(|(_, lever)| lever.creature_key.as_deref() == Some(creature));
    let Some((index, _)) = matching.next() else {
        return (None, None);
    };
    if matching.next().is_none() {
        (Some(index), None)
    } else {
        // Scarlett's action 40003 exists in both donor trees. Only the Global
        // registration has the absolute Cobra Bastion map coordinates.
        if creature == "oteryn:creature.scarlett_etzel" {
            let index = levers.iter().position(|lever| {
                lever.path
                    == "data-global/scripts/quests/grave_danger/cobra_bastion/actions_scarlett.lua"
                    && lever.lever_positions.first() == Some(&[33395, 32661, 6])
            });
            if index.is_some() {
                return (
                    index,
                    Some(
                        "Crystal201189f8:data-global/scripts/quests/grave_danger/cobra_bastion/actions_scarlett.lua:100",
                    ),
                );
            }
        }
        (None, None)
    }
}

fn short(key: &str) -> &str {
    key.split_once('.').map_or(key, |(_, suffix)| suffix)
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
}

struct Rows<'a> {
    entry: &'a Entry,
    steps: &'static [&'static str],
    next: usize,
    lap: std::time::Instant,
    counts: &'a mut Tally,
}

impl Rows<'_> {
    fn emit(&mut self, cause: Option<&str>, detail: &str) {
        let step = self.steps[self.next];
        self.next += 1;
        let seconds = self.lap.elapsed().as_secs_f64();
        self.lap = std::time::Instant::now();
        let status = if cause.is_some() {
            self.counts.fail += 1;
            "FAIL"
        } else {
            self.counts.pass += 1;
            "PASS"
        };
        let cause = cause.map_or_else(String::new, |cause| format!(" cause={cause}"));
        evidence(&format!(
            "e2e_all scenario=BS.{}.{step} status={status} seconds={seconds:.3} mode=seeded source={} evidence={}{cause}",
            self.entry.key,
            self.entry.source,
            creature_sweep::token(detail)
        ));
    }

    fn abort(&mut self, cause: &str, detail: &str) {
        let root = self.steps[self.next];
        self.emit(Some(cause), detail);
        while self.next < self.steps.len() {
            self.emit(Some("blocked"), &format!("blocked-by-{root}"));
        }
    }
}

struct Failure {
    cause: &'static str,
    detail: String,
}

fn failure(cause: &'static str, detail: impl std::fmt::Display) -> Failure {
    Failure {
        cause,
        detail: detail.to_string(),
    }
}

pub(super) async fn run(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
) -> TestResult {
    let value = match std::env::var(BATCH_VARIABLE) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(format!("invalid {BATCH_VARIABLE}: {error}").into()),
    };
    let (start, len) = batch(value.as_deref())?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    verdict::Harness::global().progress("boss-sweep-census");
    let Catalogue { entries, levers } = census(&root)?;
    let total = entries.len();
    if start >= total {
        return Err(format!("{BATCH_VARIABLE} start {start} is beyond {total} entries").into());
    }
    let selected = &entries[start..start.saturating_add(len).min(total)];
    evidence(&format!(
        "boss_sweep census total={total} start={start} len={len} selected={} mode=seeded",
        selected.len()
    ));
    let definitions = creature_sweep::definitions(&root)?;
    // The fixture owner commits a build once per character. Reuse the highest
    // applicable authored threshold across this batch so a later entry cannot
    // require rewriting an already established progression receipt.
    let build_level = selected
        .iter()
        .filter_map(|entry| {
            let authored = levers.get(entry.lever?)?;
            Some(required_level(
                authored,
                entry.creature.as_deref().unwrap_or_default(),
            ))
        })
        .max()
        .unwrap_or(1)
        .max(COMBAT_FIXTURE_MINIMUM_LEVEL);
    evidence(&format!(
        "boss_sweep build mode=seeded vocation=knight level={build_level} gear=i7388 supplies=rope,shovel,i7643"
    ));
    let mut tally = Tally::default();
    for (offset, entry) in selected.iter().enumerate() {
        let leader_index = offset % clients.accounts.len();
        verdict::Harness::global().progress("boss-sweep-entry");
        let encounter = entry.key.strip_suffix(".encounter");
        let phase = encounter.and_then(room_actor);
        let scripted = encounter.and_then(scripted_room);
        let mut rows = Rows {
            entry,
            steps: if entry.held.is_some() {
                &["route", "spawn", "kill", "reward"]
            } else if entry.key.ends_with(".arena") && entry.lever.is_none() {
                &["entry"]
            } else if scripted.is_some() {
                &["entry", "spawn"]
            } else if phase.is_some() {
                &["ineligible", "eligible", "spawn", "phase"]
            } else {
                &STEPS
            },
            next: 0,
            lap: std::time::Instant::now(),
            counts: &mut tally,
        };
        let mut party = Vec::new();
        let work = async {
            if let Some(held) = &entry.held {
                held_journey(
                    clients,
                    world,
                    held,
                    &definitions,
                    rewards,
                    (leader_index, build_level),
                    &mut party,
                    &mut rows,
                )
                .await
            } else if let Some(scripted) = scripted {
                scripted_journey(
                    clients,
                    world,
                    scripted,
                    leader_index,
                    &mut party,
                    &mut rows,
                )
                .await
            } else if let Some(lever) = entry.lever.map(|index| &levers[index]) {
                journey(
                    clients,
                    world,
                    lever,
                    &definitions,
                    rewards,
                    (leader_index, build_level),
                    &mut party,
                    &mut rows,
                )
                .await
            } else {
                let cause = if entry.key.ends_with(".arena") {
                    "arena_unreachable"
                } else {
                    "entry_binding_missing"
                };
                Err(failure(cause, format!("{}; {}", entry.gap, entry.source)))
            }
        };
        let budget = entry
            .held
            .as_ref()
            .map_or(Duration::from_secs(240), |held| {
                Duration::from_secs(240) + held.respawn.min(Duration::from_secs(120))
            });
        let result = tokio::time::timeout(budget, work)
            .await
            .unwrap_or_else(|_| {
                Err(failure(
                    "deadline",
                    format!("entry journey exceeded {}s", budget.as_secs()),
                ))
            });
        if let Err(error) = result {
            rows.abort(error.cause, &error.detail);
        }
        for client in &mut party {
            let cleanup: TestResult = async {
                walk_to_logout_tile(client, world).await?;
                let (result, reply, _, _) = logout_retrying_busy(client).await?;
                if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
                    return Err(format!("boss fixture logout refused: {reply}").into());
                }
                Ok(())
            }
            .await;
            if let Err(error) = cleanup {
                evidence(&format!(
                    "boss_sweep cleanup={} error={}",
                    entry.key,
                    creature_sweep::token(&error.to_string())
                ));
            }
        }
    }
    evidence(&format!(
        "boss_sweep summary total={total} selected={} rows_pass={} rows_fail={} mode=seeded",
        selected.len(),
        tally.pass,
        tally.fail
    ));
    if tally.fail > 0 {
        return Err(format!("boss sweep: {} FAIL rows (see BS.*)", tally.fail).into());
    }
    Ok(())
}

fn cell([x, y, floor]: [i32; 3]) -> TestResult<TilePos> {
    // Bundle-backed Crystal rooms keep source x/y/+z (activity_template.rs:601–616).
    Ok(TilePos {
        x: u16::try_from(x)?,
        y: u16::try_from(y)?,
        floor: -i8::try_from(floor)?,
    })
}

async fn lever(
    client: &mut DevClientSession,
    payload: &[u8],
) -> TestResult<(CommandStatus, BossActivityReply)> {
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BOSS_ENCOUNTER_V1,
            wire::COMMAND_TYPE_BOSS_LEVER,
            payload,
        )
        .await?;
    let decoded = wire::decode_boss_activity_reply(&reply.payload)
        .map_err(|error| format!("lever status={:?} decode={error:?}", reply.status))?;
    Ok((reply.status, decoded))
}

async fn cooldowns(
    client: &mut DevClientSession,
    key: &str,
) -> TestResult<(u64, Option<u64>, u64)> {
    let mut after = String::new();
    let mut cooldown = None;
    let mut kills = None;
    for _ in 0..20 {
        let query = panel::query(&after).ok_or("cooldown request encoder")?;
        let response = client
            .session_mut()
            .gated_command(query.capability, query.kind, &query.payload)
            .await?;
        let snapshot = panel::decode(&response.payload)
            .filter(|_| response.status == CommandStatus::Accepted)
            .ok_or("cooldown snapshot rejected or malformed")?;
        cooldown = cooldown.or_else(|| {
            snapshot
                .cooldowns
                .iter()
                .find_map(|(entry, time)| (entry == key).then_some(*time))
        });
        kills = kills.or_else(|| {
            snapshot
                .progress
                .iter()
                .find_map(|(entry, count)| (entry == key).then_some(*count))
        });
        if let Some(next) = [snapshot.cooldown_next, snapshot.progress_next]
            .into_iter()
            .flatten()
            .min()
        {
            if next <= after {
                return Err("cooldown cursor did not advance".into());
            }
            after = next;
        } else {
            return Ok((snapshot.now, cooldown, kills.unwrap_or(0)));
        }
    }
    Err("cooldown query exceeded 20 pages".into())
}

fn cooldown_matches(before: u64, after: u64, actual: u64, seconds: u32) -> bool {
    let duration = u64::from(seconds) * 1000;
    actual > after
        && actual
            .checked_sub(duration)
            .is_some_and(|minted| (before..=after).contains(&minted))
}

async fn walk_to(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: TilePos,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = creature_sweep::far_route(world, from, |at| at == target)
        .map_err(|_| format!("no player route from {from:?} to authored lever {target:?}"))?;
    walk(client, world, &path).await?;
    if tile(client.world_spatial().actor_position)? != target {
        return Err("walk did not finish on its authored lever cell".into());
    }
    Ok(())
}

fn quest_minimum(creature: &str) -> Option<u32> {
    use oteryn_gameplay_quests::quest::{rotten_blood::REQUIRED_LEVEL, soul_war::MINIMUM_LEVEL};

    match Encounter::from_boss_key(creature)?.family() {
        QuestFamily::RottenBlood => Some(REQUIRED_LEVEL),
        QuestFamily::SoulWar => Some(MINIMUM_LEVEL),
        QuestFamily::PrimalOrdeal => None,
    }
}

fn off_lever_refusal(quest_minimum: Option<u32>, level: u32) -> BossActivityRefusal {
    // The generic endgame entry gate (endgame_entry.rs:55) checks level and
    // quest access before boss_activity.rs:417 checks the lever roster.
    if quest_minimum.is_some_and(|required| level < required) {
        BossActivityRefusal::Level
    } else {
        BossActivityRefusal::Position
    }
}

async fn refused(
    client: &mut DevClientSession,
    payload: &[u8],
    expected: BossActivityRefusal,
) -> TestResult {
    let source = client.join_snapshot().game_session_id;
    let position = client.world_spatial().actor_position;
    let reply = lever(client, payload).await?;
    if reply
        != (
            CommandStatus::Rejected,
            BossActivityReply::Refused(expected),
        )
        || source != client.join_snapshot().game_session_id
        || position != client.world_spatial().actor_position
    {
        return Err(format!(
            "expected {expected:?} without handoff; observed {reply:?}; owner:{ROOM_OWNER}"
        )
        .into());
    }
    Ok(())
}

async fn scope(
    client: &mut DevClientSession,
    prior: crate::foundation::GameSessionId,
    entry: Option<([u8; 16], TilePos)>,
    wait: Duration,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + wait;
    while client.join_snapshot().game_session_id == prior {
        if tokio::time::Instant::now() >= deadline {
            return Err("fresh activity snapshot did not arrive".into());
        }
        verdict::Harness::global().progress("boss-sweep-handoff");
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    let position = tile(client.world_spatial().actor_position)?;
    let current = client.join_snapshot().game_session_id;
    let scope = client
        .session_mut()
        .activity_scope()
        .ok_or("missing activity scope")?;
    let correct = match entry {
        Some((handoff, expected)) => {
            matches!(scope.scope, ActivityScope::Instance { .. })
                && scope.handoff == handoff
                && position == expected
        }
        None => matches!(scope.scope, ActivityScope::Channel { .. }),
    };
    if !correct || scope.source_session != *prior.as_bytes() || scope.session != *current.as_bytes()
    {
        return Err(format!("unexpected scope {scope:?} at {position:?}").into());
    }
    Ok(())
}

fn scripted_placement(world: &BundleWorld, room: &ScriptedRoom) -> TestResult<(TilePos, u64)> {
    use crate::map::view::{MapFacts, placement_key};

    let mut selected = None;
    for (floor, x, y, view) in world.base().tiles() {
        let at = TilePos { x, y, floor };
        for ordinal in 0..view.depths().iter().filter(|depth| **depth == 0).count() {
            let Some(key) = u8::try_from(ordinal)
                .ok()
                .and_then(|ordinal| placement_key(at, ordinal))
            else {
                continue;
            };
            if !world.facts().base_hidden(key)
                && world
                    .facts()
                    .quest_callback_matches(key, room.unique, room.action, None)
                && selected.replace((at, key)).is_some()
            {
                return Err("multiple published placements match donor room trigger".into());
            }
        }
    }
    selected.ok_or_else(|| "donor room trigger has no published placement in bundle".into())
}

async fn wait_at(client: &mut DevClientSession, landing: TilePos) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while tile(client.world_spatial().actor_position).ok() != Some(landing) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("trigger did not land the player at {landing:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
}

async fn scripted_journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    room: ScriptedRoom,
    leader_index: usize,
    party: &mut Vec<DevClientSession>,
    rows: &mut Rows<'_>,
) -> Result<(), Failure> {
    use crate::map::view::MapFacts;

    let (at, key) =
        scripted_placement(world, &room).map_err(|error| failure("scripted_placement", error))?;
    if room.roster.len() > clients.accounts.len() {
        return Err(failure(
            "fixture_party_size",
            "insufficient scripted room occupants",
        ));
    }
    for offset in 0..room.roster.len().max(1) {
        let index = (leader_index + offset) % clients.accounts.len();
        party.push(
            connect_character(clients, 0xb4, index)
                .await
                .map_err(|error| failure("fixture", error))?,
        );
    }
    for (client, &stand) in party.iter_mut().zip(room.roster) {
        walk_to(
            client,
            world,
            cell(stand).map_err(|error| failure("content", error))?,
        )
        .await
        .map_err(|error| failure("entry_route", error))?;
    }
    let leader = &mut party[0];
    if room.roster.is_empty() {
        let from = tile(leader.world_spatial().actor_position)
            .map_err(|error| failure("entry_route", error))?;
        let path = creature_sweep::far_route(world, from, |pos| {
            pos.floor == at.floor && pos.x.abs_diff(at.x).max(pos.y.abs_diff(at.y)) <= 1
        })
        .map_err(|_| {
            failure(
                "entry_route",
                format!("no route to scripted trigger {at:?}"),
            )
        })?;
        walk(leader, world, &path)
            .await
            .map_err(|error| failure("entry_route", error))?;
    }
    let prior = leader
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .map(|entity| entity.entity)
        .collect::<Vec<_>>();
    if let Some(landing) = room.step_landing {
        // The final wire step can teleport before walk_to sees the trigger tile.
        let from = tile(leader.world_spatial().actor_position)
            .map_err(|error| failure("entry_route", error))?;
        let path = creature_sweep::far_route(world, from, |pos| pos == at)
            .map_err(|_| failure("entry_route", format!("no route onto seal at {at:?}")))?;
        let (&step, approach) = path
            .split_last()
            .ok_or_else(|| failure("entry_route", "fixture already occupies seal"))?;
        walk(leader, world, approach)
            .await
            .map_err(|error| failure("entry_route", error))?;
        let reply = leader
            .step_retrying(step)
            .await
            .map_err(|error| failure("scripted_step", error))?;
        if reply.disposition != StepDisposition::Moved {
            return Err(failure(
                "scripted_step",
                format!("seal step: {:?}", reply.disposition),
            ));
        }
        wait_at(
            leader,
            cell(landing).map_err(|error| failure("content", error))?,
        )
        .await
        .map_err(|error| failure("scripted_step", error))?;
    } else {
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let reply = leader
            .use_object(&placement, world.facts().object_revision(key))
            .await
            .map_err(|error| failure("scripted_use", error))?;
        if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
            return Err(failure(
                "scripted_use",
                format!("room trigger: {:?}", reply.disposition),
            ));
        }
    }
    rows.emit(None, "authored-trigger-committed");
    let spawn = room
        .spawn
        .map(cell)
        .transpose()
        .map_err(|error| failure("content", error))?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        if leader.world_entities().is_some_and(|entities| {
            entities.iter().any(|entity| {
                !prior.contains(&entity.entity)
                    && creature_sweep::creature_named(entity, room.actor)
                    && spawn.is_none_or(|at| tile(entity.position).ok() == Some(at))
            })
        }) {
            rows.emit(None, &format!("scripted-actor-visible:{}", room.actor));
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(failure(
                "spawn_absent",
                format!("no new {} frame", room.actor),
            ));
        }
        leader
            .service_liveness(Duration::from_millis(50))
            .await
            .map_err(|error| failure("spawn_protocol", error))?;
        leader.take_events();
    }
}

async fn observe_boss(
    client: &mut DevClientSession,
    name: &str,
    position: TilePos,
    wait: Duration,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    let deadline = tokio::time::Instant::now() + wait;
    loop {
        if let Some(target) = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entity| {
                    creature_sweep::creature_named(entity, name)
                        && tile(entity.position).ok() == Some(position)
                })
                .copied()
        }) {
            return Ok(target);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no {name} frame at authored {position:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn start_attack(
    client: &mut DevClientSession,
    target: oteryn_dev_client::WorldSpatialEntity,
) -> TestResult {
    let modes = client
        .session_mut()
        .set_fight_modes(&attack::FightModes {
            chase: attack::ChaseMode::Chase,
            ..attack::FightModes::DEFAULT
        })
        .await?;
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if modes.disposition != attack::AttackIntentDisposition::Ok
        || attack.status != CommandStatus::Accepted
        || attack.disposition != attack::AttackIntentDisposition::Ok
    {
        return Err(format!("native combat refused: {modes:?}; {attack:?}").into());
    }
    Ok(())
}

async fn observe_room_actor(
    client: &mut DevClientSession,
    target: oteryn_dev_client::WorldSpatialEntity,
    name: &str,
) -> TestResult {
    let instance = client.join_snapshot().game_session_id;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(40);
    let mut attacking = false;
    loop {
        if client.join_snapshot().game_session_id != instance {
            return Err(format!("left parent room before observing {name}").into());
        }
        if client.world_entities().is_some_and(|entities| {
            entities
                .iter()
                .any(|entity| creature_sweep::creature_named(entity, name))
        }) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no {name} frame in parent room").into());
        }
        if !attacking {
            start_attack(client, target).await?;
            attacking = true;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn fight(
    client: &mut DevClientSession,
    target: oteryn_dev_client::WorldSpatialEntity,
) -> TestResult<ActorPosition> {
    start_attack(client, target).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut position = target.position;
    while let Some(visible) = client
        .world_entities()
        .and_then(|entities| entities.get(&target.entity))
    {
        position = visible.position;
        if tokio::time::Instant::now() >= deadline {
            return Err("boss remained visible after 120s of native combat".into());
        }
        verdict::Harness::global().progress("boss-sweep-combat");
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(position)
}

async fn verify_experience(
    client: &mut DevClientSession,
    creature: &str,
    base_experience: u64,
    selection: &oteryn_protocol_oteryn::daily_boost::DailyBoostView,
    (before, level): (u64, u32),
) -> Result<u64, Failure> {
    // The free-account fixture starts with full stamina and the leader contributes the damage.
    let pool = client_daily_boost::boosted_xp(base_experience, creature, selection)
        .map_err(|error| failure("content", error))?;
    let pool = i64::try_from(pool).map_err(|error| failure("content", error))?;
    let expected = oteryn_gameplay_characters::hunting_experience::hunting_experience(
        pool, 0, 0, 2520, false, level,
    )
    .map_err(|error| failure("content", format!("hunting_experience: {error:?}")))?;
    let expected = u64::try_from(expected).map_err(|error| failure("content", error))?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let (after, _, _) = creature_sweep::read_stats(client)
            .await
            .map_err(|error| failure("protocol", error))?;
        if after.checked_sub(before) == Some(expected) {
            return Ok(expected);
        }
        if after.saturating_sub(before) > expected || tokio::time::Instant::now() >= deadline {
            return Err(failure(
                "xp_mismatch",
                format!("before:{before};after:{after};expected-delta:{expected}"),
            ));
        }
        verdict::Harness::global().progress("boss-sweep-xp");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn chest(client: &mut DevClientSession) -> TestResult<panel::RewardChest> {
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BOSS_ENCOUNTER_V1,
            wire::COMMAND_TYPE_BOSS_REWARD_OPEN,
            &[],
        )
        .await?;
    panel::decode_rewards(&response.payload)
        .filter(|_| response.status == CommandStatus::Accepted)
        .ok_or_else(|| format!("private chest refused: {:?}", response.status).into())
}

/// The pinned loot rows are the draw owner (boss_reward_settlement.rs:230–250).
/// Each observed item needs an authored identity and an attainable sum of rolls.
type LootBounds = BTreeMap<u32, Vec<(u64, u64)>>;

fn loot_matches(allowed: &LootBounds, observed: &BTreeMap<u32, u64>) -> bool {
    observed.iter().all(|(item, &count)| {
        count > 0
            && allowed
                .get(item)
                .is_some_and(|bounds| creature_sweep::possible_loot_count(bounds, count))
    })
}

fn allowed_loot(
    content: &crate::content::creature_reward::CreatureRewardRow,
    items: &crate::content::item_ref::ItemDefinitionIndex,
    private: bool,
) -> TestResult<LootBounds> {
    let mut allowed = LootBounds::new();
    for entry in &content.loot_table.entries {
        let item = &entry.item;
        let reference = items
            .definition_ref("Item", &item.production_key, &item.revision_ref)
            .ok_or("authored loot item lacks a wire reference")?;
        // The corpse (loot_plan.rs:375) and personal (loot_plan/boss.rs:130) owners
        // materialize containers and charged items with their native stack counts.
        let details = &entry.details;
        let bounds = if details.container_capacity.is_some() || (private && !details.stackable) {
            (1, 1)
        } else if let Some(charges) = details.charges {
            let count = if details.stackable {
                u64::from(charges)
            } else {
                1
            };
            (count, count)
        } else {
            (u64::from(entry.min_count), u64::from(entry.max_count))
        };
        allowed.entry(reference.get()).or_default().push(bounds);
    }
    Ok(allowed)
}

async fn physical_loot(
    client: &mut DevClientSession,
    content: &crate::content::creature_reward::CreatureRewardRow,
    position: ActorPosition,
    existing: &std::collections::BTreeSet<oteryn_dev_client::EntityRef>,
) -> TestResult<String> {
    let items = provisioning::item_index()?;
    let corpse = content
        .corpse_item
        .as_ref()
        .ok_or("no authored corpse item")?;
    let reference = items
        .definition_ref("Item", &corpse.production_key, &corpse.revision_ref)
        .ok_or("authored corpse lacks a wire reference")?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let handle = loop {
        if let Some(handle) = client.world_entities().and_then(|entities| {
            entities.iter().find_map(|entity| {
                if entity.kind != EntityKind::Corpse
                    || entity.position != position
                    || existing.contains(&entity.entity)
                {
                    return None;
                }
                match entity.detail {
                    EntityDetail::Object {
                        item_definition_ref,
                        item_handle,
                        ..
                    }
                    | EntityDetail::ObjectWithAppearance {
                        item_definition_ref,
                        item_handle,
                        ..
                    } if item_definition_ref == reference.get() => item_handle,
                    _ => None,
                }
            })
        }) {
            break handle;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("no new authored corpse at the boss death tile".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    let observed: BTreeMap<_, _> = creature_sweep::open_corpse(client, handle)
        .await?
        .into_iter()
        .map(|(key, count)| (key.get(), count))
        .collect();
    if !loot_matches(&allowed_loot(content, &items, false)?, &observed) {
        return Err(format!("corpse loot differs from pinned table: {observed:?}").into());
    }
    Ok(format!("corpse-items:{}", observed.len()))
}

async fn reward(
    client: &mut DevClientSession,
    definition: &creature_sweep::Definition,
    creature: &str,
    prior: &panel::RewardChest,
    content: &crate::content::creature_reward::CreatureRewardTable,
) -> TestResult<String> {
    use oteryn_dev_client::client_runtime::boss_rewards::{
        self, BossRewardsCommand, BossRewardsReply,
    };
    use oteryn_protocol_oteryn::boss_rewards::{BossRewardStatus, BrowseRequest};

    let content = content
        .row(creature)
        .map_err(|error| format!("missing pinned reward for {creature}: {error:?}"))?;
    let key = content
        .boss
        .as_ref()
        .map_or(creature, |boss| boss.key.as_str());
    let opened = chest(client).await?;
    let earned = opened.entries.iter().find(|entry| {
        entry.boss_key == key
            && entry.minted_at_unix_ms >= prior.now
            && !prior
                .entries
                .iter()
                .any(|previous| previous.death == entry.death)
    });
    if !definition.reward_boss {
        // These bosses use ordinary corpse loot. A private group here would be the wrong route;
        // the caller checks the physical corpse before the activity return unloads it.
        return if earned.is_none() {
            Ok("no-private-reward-for-ordinary-corpse".into())
        } else {
            Err("ordinary corpse boss minted a private group".into())
        };
    }
    let earned = earned.ok_or("private group missing for this boss death")?;
    let items = provisioning::item_index()?;
    let allowed = allowed_loot(&content, &items, true)?;
    let mut observed = BTreeMap::new();
    let mut rows = 0;
    let mut after_ordinal = 0;
    loop {
        let result = boss_rewards::send(
            client.session_mut(),
            &BossRewardsCommand::Browse(BrowseRequest {
                death: earned.death,
                after_ordinal,
            }),
        )
        .await?;
        let Some(BossRewardsReply::Browse(reply)) = result.reply else {
            return Err("earned chest has no browse reply".into());
        };
        if reply.status != BossRewardStatus::Ok
            || reply.death != earned.death
            || reply.expires_at_unix_ms != earned.expires_at_unix_ms
        {
            return Err(format!("earned chest browse returned {:?}", reply.status).into());
        }
        let previous = after_ordinal;
        for row in reply.rows {
            if row.ordinal <= after_ordinal {
                return Err("earned chest browse ordinals did not advance".into());
            }
            after_ordinal = row.ordinal;
            *observed.entry(row.definition_reference).or_default() += u64::from(row.quantity);
            rows += 1;
        }
        if !reply.more {
            break;
        }
        if rows >= earned.item_count || previous == after_ordinal {
            return Err("earned chest browse exceeded advertised item count".into());
        }
    }
    if rows != earned.item_count || !loot_matches(&allowed, &observed) {
        return Err(format!(
            "private loot differs from pinned table: rows={rows}; observed={observed:?}"
        )
        .into());
    }
    Ok(format!("private-chest-items:{rows}"))
}

type Definitions = BTreeMap<String, (creature_sweep::Definition, Option<String>)>;

fn required_level(authored: &Lever, creature: &str) -> u32 {
    authored
        .minimum_level
        .into_iter()
        .chain(quest_minimum(creature))
        .max()
        .unwrap_or(1)
}

const KOSHEI_HELD: TilePos = TilePos {
    x: 33271,
    y: 32406,
    floor: -12,
};
const KOSHEI_SWITCH: TilePos = TilePos {
    x: 33281,
    y: 32447,
    floor: -8,
};
const KOSHEI_ENTRY_SOURCE: &str =
    "content/quests/authored-bindings/koshei_the_deathless_journey.json:62";

async fn write_quest_access(url: &str, character: &[u8; 16], entry: &Entry) -> TestResult {
    use oteryn_gameplay_quests::quest::{
        rotten_blood::ACCESS_TRACK,
        soul_war::{ALL_MINI_BOSSES, TAINT_DURATION_SECONDS},
    };

    let encounter = entry.creature.as_deref().and_then(Encounter::from_boss_key);
    let family = encounter.map(Encounter::family);
    let final_boss = encounter.is_some_and(Encounter::is_final);
    let rotten_access = family == Some(QuestFamily::RottenBlood);
    let soul_access = family == Some(QuestFamily::SoulWar);
    let rotten_final = rotten_access && final_boss;
    let soul_final = soul_access && final_boss;
    let koshei_access = entry.koshei_switch();
    if !rotten_access && !soul_access && !koshei_access {
        return Ok(());
    }

    // The owner tests exercise both the unseeded rejection and the accepted
    // prerequisite states in gameplay_transport/postgres_tests/boss_activity.rs:810.
    let mut connection = sqlx::PgConnection::connect(url).await?;
    let mut transaction = connection.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *transaction)
        .await?;
    if rotten_access {
        sqlx::query(
            "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
             SELECT character_id,$2,'oteryn:quest/rotten-blood',4,character_revision
             FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
             ON CONFLICT DO NOTHING",
        )
        .bind(character.as_slice())
        .bind(ACCESS_TRACK)
        .execute(&mut *transaction)
        .await?;
    }
    if rotten_final {
        sqlx::query(
            "INSERT INTO game_character_rotten_blood(character_id,child_flags)
             VALUES(encode($1,'hex')::uuid,15) ON CONFLICT (character_id) DO UPDATE
             SET child_flags = game_character_rotten_blood.child_flags | excluded.child_flags",
        )
        .bind(character.as_slice())
        .execute(&mut *transaction)
        .await?;
    }
    if koshei_access {
        // The dispatch owner drives the complete quest in
        // postgres_tests/support/koshei_the_deathless_journey_fixtures.rs.
        for (track, value) in [
            (
                "oteryn:quest-progress/chosen-source/koshei_the_deathless_quest/s1",
                4,
            ),
            ("oteryn:quest-progress/qnpcfx/koshei/amulet", 2),
        ] {
            sqlx::query(
                "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
                 SELECT character_id,$2,'oteryn:quest.koshei_the_deathless_quest',$3,character_revision
                 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
                 ON CONFLICT DO NOTHING",
            )
            .bind(character.as_slice())
            .bind(track)
            .bind(i64::from(value))
            .execute(&mut *transaction)
            .await?;
        }
    }
    if soul_access {
        // A fresh taint clock is part of the final room gate. The owner proves
        // expiry rejection, then seeds the database clock (boss_activity.rs:849–864).
        let defeated = if soul_final {
            i16::from(ALL_MINI_BOSSES)
        } else {
            0
        };
        sqlx::query(
            "INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at)
             VALUES(encode($1,'hex')::uuid,true,$2,
             CASE WHEN $2 > 0 THEN floor(extract(epoch FROM transaction_timestamp()))::bigint END)
             ON CONFLICT (character_id) DO UPDATE SET
             teleport_access=true,
             defeated_bosses=GREATEST(game_character_soul_war.defeated_bosses, excluded.defeated_bosses),
             first_taint_at=CASE
                WHEN excluded.first_taint_at IS NULL THEN game_character_soul_war.first_taint_at
                WHEN game_character_soul_war.first_taint_at > excluded.first_taint_at - $3
                THEN game_character_soul_war.first_taint_at
                ELSE excluded.first_taint_at
             END
             WHERE excluded.defeated_bosses > 0 OR NOT game_character_soul_war.teleport_access",
        )
        .bind(character.as_slice())
        .bind(defeated)
        .bind(TAINT_DURATION_SECONDS)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    evidence(&format!(
        "boss_sweep quest_access mode=seeded entry={} rotten_blood={rotten_access} rotten_blood_final={rotten_final} soul_war={soul_access} soul_war_final={soul_final} koshei={koshei_access} koshei_source={KOSHEI_ENTRY_SOURCE} oracle=crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/boss_activity.rs:810",
        entry.key
    ));
    Ok(())
}

async fn seed_quest_access(clients: &SeamClients<'_>, index: usize, entry: &Entry) -> TestResult {
    tokio::time::timeout(
        Duration::from_secs(10),
        write_quest_access(clients.url, &clients.characters[index], entry),
    )
    .await
    .map_err(|_| "quest access fixture deadline")?
}

async fn walk_to_logout_tile(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let no_logout = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .map(|tile| tile.no_logout())
    };
    if no_logout(from) == Some(true) {
        let path = route(world, from, |at| no_logout(at) == Some(false))
            .ok_or("no walkable tile that allows fixture logout")?;
        walk(client, world, &path).await?;
    }
    Ok(())
}

async fn seeded_knight(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    index: usize,
    level: u32,
    entry: &Entry,
) -> TestResult<DevClientSession> {
    walk_to_logout_tile(&mut client, world).await?;
    let client = provisioning::provision_boss(client, clients, index, level).await?;
    // The owner has now committed progression. Quest tracks need a real
    // character revision >= 2 (0056_character_quest_state.sql:100).
    seed_quest_access(clients, index, entry).await?;
    Ok(client)
}

#[allow(
    clippy::too_many_arguments,
    reason = "shared world, fixture, reward, and row owners"
)]
async fn held_journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    held: &HeldSpawn,
    definitions: &Definitions,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
    (leader_index, build_level): (usize, u32),
    party: &mut Vec<DevClientSession>,
    rows: &mut Rows<'_>,
) -> Result<(), Failure> {
    let creature = rows
        .entry
        .creature
        .as_deref()
        .ok_or_else(|| failure("content", "missing held creature"))?;
    let (definition, _) = definitions
        .get(creature)
        .ok_or_else(|| failure("content", "missing held creature definition"))?;
    let target = held.at;
    if world
        .base()
        .tile(target.x, target.y, target.floor)
        .is_none()
    {
        return Err(failure(
            "world_route",
            format!("held spawn {target:?} lies outside the loaded world bundle"),
        ));
    }
    let player = connect_character(clients, 0xb4, leader_index)
        .await
        .map_err(|error| failure("fixture", error))?;
    let player = seeded_knight(
        player,
        clients,
        world,
        leader_index,
        build_level,
        rows.entry,
    )
    .await
    .map_err(|error| failure("fixture", error))?;
    party.push(player);
    let player = &mut party[0];
    let from =
        tile(player.world_spatial().actor_position).map_err(|error| failure("fixture", error))?;
    let koshei_switch = rows.entry.koshei_switch();
    let path = creature_sweep::far_route(world, from, |at| {
        if koshei_switch {
            at.x == KOSHEI_SWITCH.x - 1
                && at.y == KOSHEI_SWITCH.y
                && at.floor == KOSHEI_SWITCH.floor
        } else {
            at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) == 1
        }
    });
    let path = match path {
        Ok(path) => path,
        Err(_) => {
            return Err(failure(
                "world_route",
                format!(
                    "no native route from {from:?} to {}",
                    if koshei_switch {
                        format!("tomb switch {KOSHEI_SWITCH:?}")
                    } else {
                        format!("held spawn {target:?}")
                    }
                ),
            ));
        }
    };
    walk(player, world, &path)
        .await
        .map_err(|error| failure("world_route", error))?;
    if koshei_switch {
        use crate::map::view::MapFacts;

        rows.emit(None, "walked-to-authored-tomb-switch");
        let key = crate::map::view::placement_key(KOSHEI_SWITCH, 1)
            .ok_or_else(|| failure("content", KOSHEI_ENTRY_SOURCE))?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let reply = player
            .use_object(&placement, world.facts().object_revision(key))
            .await
            .map_err(|error| failure("world_spawn_trigger", error))?;
        if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
            return Err(failure(
                "world_spawn_trigger",
                format!(
                    "Koshei switch: {:?}; {KOSHEI_ENTRY_SOURCE}",
                    reply.disposition
                ),
            ));
        }
        wait_at(
            player,
            TilePos {
                x: 33270,
                y: 32406,
                floor: -12,
            },
        )
        .await
        .map_err(|error| failure("world_spawn_trigger", error))?;
    } else {
        rows.emit(None, "walked-to-authored-world-spawn");
    }
    let wait = held.respawn.min(Duration::from_secs(120));
    let target = observe_boss(player, &definition.name, target, wait)
        .await
        .map_err(|error| {
            failure(
                "world_spawn",
                format!(
                    "{error}; observed {}ms of authored {}ms respawn",
                    wait.as_millis(),
                    held.respawn.as_millis()
                ),
            )
        })?;
    rows.emit(None, &format!("world-spawn={:?}", target.entity));
    let existing = player
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entity| entity.kind == EntityKind::Corpse)
        .map(|entity| entity.entity)
        .collect();
    let baseline: TestResult<_> = async {
        let selection = client_daily_boost::reward_selection(player).await?;
        let (experience, level, _) = creature_sweep::read_stats(player).await?;
        Ok((selection, (experience, level)))
    }
    .await;
    let position = fight(player, target)
        .await
        .map_err(|error| failure("combat", error))?;
    let content = rewards
        .row(creature)
        .map_err(|error| failure("reward_content", format!("{error:?}")))?;
    let loot = physical_loot(player, &content, position, &existing)
        .await
        .map_err(|error| failure("corpse_unconfirmed", error))?;
    rows.emit(None, "target-removed-with-new-pinned-corpse");
    let (selection, before) = baseline.map_err(|error| failure("protocol", error))?;
    let expected =
        verify_experience(player, creature, definition.experience, &selection, before).await?;
    rows.emit(None, &format!("{loot};xp:{expected}"));
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "shared world, fixture, reward, and row owners"
)]
async fn journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    authored: &Lever,
    definitions: &Definitions,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
    (leader_index, build_level): (usize, u32),
    party: &mut Vec<DevClientSession>,
    rows: &mut Rows<'_>,
) -> Result<(), Failure> {
    let creature = rows.entry.creature.as_deref().ok_or_else(|| {
        failure(
            "entry_binding_missing",
            "source lever has no creature identity",
        )
    })?;
    let boss_key = format!("oteryn:boss.{}", short(creature));
    let payload = wire::encode_boss_key(&boss_key)
        .map_err(|error| failure("content", format!("boss key encoder: {error:?}")))?;
    let Some((definition, _)) = definitions.get(creature) else {
        return Err(failure("content", "missing authored creature definition"));
    };
    party.push(
        connect_character(clients, 0xb4, leader_index)
            .await
            .map_err(|error| failure("fixture", error))?,
    );
    let leader = &mut party[0];
    // The negative case has an ordinary admitted character off the authored lever.
    // The owner tests the command sender's presence in the lever roster (boss_activity.rs:417).
    let from =
        tile(leader.world_spatial().actor_position).map_err(|error| failure("fixture", error))?;
    let levers = authored
        .lever_positions
        .iter()
        .copied()
        .map(cell)
        .collect::<TestResult<Vec<_>>>()
        .map_err(|error| failure("content", error))?;
    if levers.contains(&from) {
        let path = route(world, from, |at| !levers.contains(&at))
            .ok_or_else(|| failure("fixture", "no adjacent cell outside the lever roster"))?;
        walk(leader, world, &path)
            .await
            .map_err(|error| failure("fixture", error))?;
    }
    let (_, initial_cooldown, _) = cooldowns(leader, &boss_key)
        .await
        .map_err(|error| failure("protocol", error))?;
    let expected = if let Some(minimum) = quest_minimum(creature) {
        let (_, level, _) = creature_sweep::read_stats(leader)
            .await
            .map_err(|error| failure("protocol", error))?;
        if level >= minimum {
            seed_quest_access(clients, leader_index, rows.entry)
                .await
                .map_err(|error| failure("fixture", format!("quest access seed: {error}")))?;
        }
        off_lever_refusal(Some(minimum), level)
    } else {
        BossActivityRefusal::Position
    };
    let origin = leader.join_snapshot().game_session_id;
    let before_position = leader.world_spatial().actor_position;
    let negative = lever(leader, &payload)
        .await
        .map_err(|error| failure("entry_refusal", error))?;
    if !matches!(
        negative,
        (CommandStatus::Rejected, BossActivityReply::Refused(_))
    ) || origin != leader.join_snapshot().game_session_id
        || before_position != leader.world_spatial().actor_position
    {
        return Err(failure(
            "entry_refusal",
            format!("unsafe to continue after {negative:?}"),
        ));
    }
    let (_, negative_cooldown, _) = cooldowns(leader, &boss_key)
        .await
        .map_err(|error| failure("protocol", error))?;
    if initial_cooldown != negative_cooldown {
        return Err(failure(
            "entry_refusal",
            "ineligible attempt changed cooldown",
        ));
    }
    if negative.1 == BossActivityReply::Refused(expected) {
        rows.emit(
            None,
            &format!("typed-{expected:?}-without-handoff-or-cooldown-change"),
        );
    } else {
        rows.emit(
            Some("entry_refusal"),
            &format!("expected {expected:?}; observed {negative:?}; owner:{ROOM_OWNER}"),
        );
    }

    let minimum = authored
        .minimum_players
        .filter(|&size| size > 0 && size <= levers.len())
        .ok_or_else(|| failure("content", "no valid authored party minimum"))?;
    if minimum > clients.accounts.len() {
        return Err(failure(
            "fixture_party_size",
            format!("requires {minimum} distinct lever occupants"),
        ));
    }
    let minimum_level = required_level(authored, creature);
    let leader = party
        .pop()
        .ok_or_else(|| failure("fixture", "no admitted leader"))?;
    party.push(
        seeded_knight(
            leader,
            clients,
            world,
            leader_index,
            build_level,
            rows.entry,
        )
        .await
        .map_err(|error| failure("fixture", error))?,
    );
    for offset in 1..minimum {
        let index = (leader_index + offset) % clients.accounts.len();
        let member = connect_character(clients, 0xb5, index)
            .await
            .map_err(|error| failure("fixture", error))?;
        party.push(
            seeded_knight(member, clients, world, index, build_level, rows.entry)
                .await
                .map_err(|error| failure("fixture", error))?,
        );
    }

    for (client, &at) in party.iter_mut().zip(&levers) {
        let (_, level, _) = creature_sweep::read_stats(client)
            .await
            .map_err(|error| failure("fixture", error))?;
        if level < minimum_level {
            return Err(failure(
                "fixture_level",
                format!("authored level {minimum_level}; fixture level {level}"),
            ));
        }
        walk_to(client, world, at)
            .await
            .map_err(|error| failure("entry_route", error))?;
    }
    let (leader, others) = party
        .split_first_mut()
        .ok_or_else(|| failure("fixture", "no leader"))?;
    let selection = client_daily_boost::reward_selection(leader)
        .await
        .map_err(|error| failure("protocol", error))?;
    let (before_xp, level, _) = creature_sweep::read_stats(leader)
        .await
        .map_err(|error| failure("protocol", error))?;
    let before_chest = chest(leader)
        .await
        .map_err(|error| failure("protocol", error))?;
    let (before, active, kills) = cooldowns(leader, &boss_key)
        .await
        .map_err(|error| failure("protocol", error))?;
    if active.is_some_and(|deadline| deadline > before) {
        return Err(failure(
            "fixture_cooldown",
            "fixture already has an active cooldown for this room",
        ));
    }
    let origin = leader.join_snapshot().game_session_id;
    let other_sessions: Vec<_> = others
        .iter()
        .map(|client| client.join_snapshot().game_session_id)
        .collect();
    let reply = lever(leader, &payload)
        .await
        .map_err(|error| failure("entry_protocol", error))?;
    let (CommandStatus::Accepted, BossActivityReply::Accepted { handoff, .. }) = reply else {
        return Err(failure(
            "entry_refused",
            format!("eligible attempt: {reply:?}; owner:{ROOM_OWNER}"),
        ));
    };
    let entry_cells = authored
        .entry_positions
        .iter()
        .copied()
        .map(cell)
        .collect::<TestResult<Vec<_>>>()
        .map_err(|error| failure("content", error))?;
    if entry_cells.len() < minimum {
        return Err(failure(
            "content",
            "fewer entry cells than required participants",
        ));
    }
    let mut cells = entry_cells.into_iter();
    let first = cells
        .next()
        .ok_or_else(|| failure("content", "missing leader entry cell"))?;
    scope(
        leader,
        origin,
        Some((handoff, first)),
        Duration::from_secs(15),
    )
    .await
    .map_err(|error| failure("entry_handoff", error))?;
    for ((member, prior), at) in others.iter_mut().zip(other_sessions).zip(cells) {
        scope(member, prior, Some((handoff, at)), Duration::from_secs(15))
            .await
            .map_err(|error| failure("entry_handoff", error))?;
    }
    rows.emit(
        None,
        &format!("native-instance-handoff;party:{minimum};minimum-level:{minimum_level}"),
    );

    let boss_at = authored
        .boss_position
        .ok_or_else(|| failure("content", "missing authored boss tile"))?;
    let boss_at = cell(boss_at).map_err(|error| failure("content", error))?;
    let target = observe_boss(leader, &definition.name, boss_at, Duration::from_secs(15))
        .await
        .map_err(|error| failure("spawn_absent", error))?;
    rows.emit(
        None,
        &format!("native-entity:{:?};at:{boss_at:?}", target.entity),
    );

    if let Some((_, name)) = rows
        .entry
        .key
        .strip_suffix(".encounter")
        .and_then(room_actor)
    {
        observe_room_actor(leader, target, name)
            .await
            .map_err(|error| failure("phase_not_observed", error))?;
        rows.emit(None, &format!("room-actor-visible:{name}"));
        return Ok(());
    }

    let instance = leader.join_snapshot().game_session_id;
    let other_instances: Vec<_> = others
        .iter()
        .map(|client| client.join_snapshot().game_session_id)
        .collect();
    let existing = leader
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entity| entity.kind == EntityKind::Corpse)
        .map(|entity| entity.entity)
        .collect();
    let death_position = fight(leader, target)
        .await
        .map_err(|error| failure("combat", error))?;
    let corpse = if definition.reward_boss {
        Ok(None)
    } else {
        match rewards.row(creature) {
            Ok(content) => physical_loot(leader, &content, death_position, &existing)
                .await
                .map(Some),
            Err(error) => Err(format!("missing pinned reward: {error:?}").into()),
        }
    };
    // A missing entity alone can be a visibility change. Require the return plus independent
    // proof from the fresh corpse, credited progress or the private reward owner.
    let return_wait =
        Duration::from_secs(u64::from(authored.after_kill_seconds.unwrap_or(60)) + 20);
    scope(leader, instance, None, return_wait)
        .await
        .map_err(|error| failure("death_unconfirmed", error))?;
    for (member, prior) in others.iter_mut().zip(other_instances) {
        scope(member, prior, None, Duration::from_secs(15))
            .await
            .map_err(|error| failure("death_unconfirmed", error))?;
    }
    let (_, _, after_kills) = cooldowns(leader, &boss_key)
        .await
        .map_err(|error| failure("protocol", error))?;
    let earned = chest(leader)
        .await
        .map_err(|error| failure("protocol", error))?;
    let reward_key = rewards
        .row(creature)
        .ok()
        .and_then(|row| row.boss.as_ref().map(|boss| boss.key.clone()));
    if !matches!(&corpse, Ok(Some(_)))
        && after_kills <= kills
        && !earned.entries.iter().any(|entry| {
            (entry.boss_key == boss_key
                || entry.boss_key == creature
                || reward_key.as_ref() == Some(&entry.boss_key))
                && !before_chest
                    .entries
                    .iter()
                    .any(|old| old.death == entry.death)
        })
    {
        return Err(failure(
            "death_unconfirmed",
            "boss disappeared without credited kill or earned chest",
        ));
    }
    rows.emit(None, "target-removal-with-return-and-credited-death");

    let reward_result: Result<_, Failure> = async {
        let expected = verify_experience(
            leader,
            creature,
            definition.experience,
            &selection,
            (before_xp, level),
        )
        .await?;
        if definition.reward_boss {
            super::full_row_w03::approach_chest(leader, clients)
                .await
                .map_err(|error| failure("reward_chest_route", error))?;
        }
        let reward = reward(leader, definition, creature, &before_chest, rewards)
            .await
            .map_err(|error| failure("reward_mismatch", error))?;
        let corpse = corpse
            .map_err(|error| failure("corpse_unverified", error))?
            .unwrap_or_default();
        Ok(format!("{reward};{corpse};xp:{expected}"))
    }
    .await;
    match reward_result {
        Ok(detail) => rows.emit(None, &detail),
        Err(error) => rows.emit(Some(error.cause), &error.detail),
    }

    let seconds = authored
        .cooldown_seconds
        .ok_or_else(|| failure("content", "no authored cooldown"))?;
    let (now, actual, credited) = cooldowns(leader, &boss_key)
        .await
        .map_err(|error| failure("protocol", error))?;
    if !actual.is_some_and(|actual| cooldown_matches(before, now, actual, seconds)) {
        return Err(failure(
            "cooldown_mismatch",
            format!(
                "before:{before};now:{now};deadline:{actual:?};authored-seconds:{seconds};owner:{COOLDOWN_OWNER}"
            ),
        ));
    }
    rows.emit(None, &format!("recorded-authored-duration:{seconds}"));
    walk_to(leader, world, levers[0])
        .await
        .map_err(|error| failure("repeat_route", error))?;
    for (client, &at) in others.iter_mut().zip(&levers[1..]) {
        walk_to(client, world, at)
            .await
            .map_err(|error| failure("repeat_route", error))?;
    }
    refused(leader, &payload, BossActivityRefusal::Cooldown)
        .await
        .map_err(|error| failure("cooldown_refusal", error))?;
    let (_, replay_deadline, replay_kills) = cooldowns(leader, &boss_key)
        .await
        .map_err(|error| failure("cooldown_replay_snapshot", error))?;
    if (replay_deadline, replay_kills) != (actual, credited) {
        return Err(failure(
            "cooldown_replay_mutation",
            format!("before:{actual:?}/{credited};after:{replay_deadline:?}/{replay_kills}"),
        ));
    }
    rows.emit(None, "typed-Cooldown-without-second-handoff-or-mutation");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_requires_positive_decimal_len() -> TestResult {
        assert_eq!(batch(None)?, (0, 3));
        assert_eq!(batch(Some("0:20"))?, (0, 20));
        for invalid in ["", "0:0", "-1:2", "+1:2", "1: 2", "1:2:3", "1:"] {
            assert!(batch(Some(invalid)).is_err(), "{invalid}");
        }
        Ok(())
    }

    #[test]
    fn census_is_sorted_and_preserves_distinct_source_records() -> TestResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let catalogue = census(&root)?;
        for class in [".encounter", ".arena", ".held.", ".lever."] {
            assert!(
                catalogue
                    .entries
                    .iter()
                    .any(|entry| entry.key.contains(class)),
                "{class}"
            );
        }
        assert!(
            catalogue
                .entries
                .windows(2)
                .all(|pair| pair[0].key < pair[1].key)
        );
        assert_eq!(
            catalogue
                .entries
                .iter()
                .filter(|entry| entry.key.starts_with("koshei_the_deathless.held."))
                .count(),
            2
        );
        Ok(())
    }

    #[test]
    fn crystal_room_bindings_preserve_parent_boss_identity() -> TestResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let catalogue = census(&root)?;
        for (key, creature) in [
            ("crystal_herald_of_fire", "oteryn:creature.herald_of_fire"),
            ("crystal_lord_retro", "oteryn:creature.lord_retro"),
            (
                "field_fill_goshnar_s_malice",
                "oteryn:creature.goshnar_s_malice",
            ),
            ("ghulosh", "oteryn:creature.ghulosh"),
        ] {
            let entry = catalogue
                .entries
                .iter()
                .find(|entry| entry.key == format!("{key}.encounter"))
                .ok_or("missing aliased encounter")?;
            assert_eq!(entry.creature.as_deref(), Some(creature));
            assert!(entry.lever.is_some());
            assert!(entry.source.starts_with("Crystal201189f8:"));
        }
        for entry in &catalogue.entries {
            let Some((parent, _)) = entry.key.strip_suffix(".encounter").and_then(room_actor)
            else {
                continue;
            };
            let lever = entry.lever.ok_or("room actor has no parent lever")?;
            assert_eq!(
                catalogue.levers[lever].creature_key.as_deref().map(short),
                Some(parent)
            );
            assert!(entry.source.starts_with("Crystal201189f8:"));
        }
        let dragon = catalogue
            .entries
            .iter()
            .find(|entry| entry.key == "the_first_dragon.encounter")
            .ok_or("missing First Dragon encounter")?;
        assert_eq!(
            dragon.creature.as_deref(),
            Some("oteryn:creature.the_first_dragon")
        );
        let dragon_lever = dragon.lever.ok_or("First Dragon encounter has no lever")?;
        assert_eq!(catalogue.levers[dragon_lever].path, SELECTED);
        let triggers = catalogue
            .entries
            .iter()
            .filter(|entry| entry.koshei_switch())
            .count();
        assert_eq!(triggers, 1);
        Ok(())
    }

    #[test]
    fn endgame_quests_check_level_before_position() {
        for creature in [
            "oteryn:creature.chagorz",
            "oteryn:creature.goshnar_s_malice",
            "oteryn:creature.goshnar_s_megalomania_purple",
        ] {
            let minimum = quest_minimum(creature);
            assert_eq!(minimum, Some(250), "{creature}");
            assert_eq!(off_lever_refusal(minimum, 249), BossActivityRefusal::Level);
            assert_eq!(
                off_lever_refusal(minimum, 250),
                BossActivityRefusal::Position
            );
        }
        assert_eq!(off_lever_refusal(None, 1), BossActivityRefusal::Position);
    }

    #[test]
    fn cooldown_requires_a_fresh_authored_duration() {
        assert!(cooldown_matches(1000, 1500, 11_250, 10));
        assert!(!cooldown_matches(1000, 1500, 10_999, 10));
        assert!(!cooldown_matches(1000, 1500, 11_501, 10));
        assert!(!cooldown_matches(1000, 1500, 1500, 0));
    }

    #[test]
    fn reward_rejects_unknown_items_and_impossible_duplicate_quantities() {
        let allowed = BTreeMap::from([(12, vec![(2, 2), (5, 5)])]);
        assert!(loot_matches(&allowed, &BTreeMap::from([(12, 7)])));
        assert!(!loot_matches(&allowed, &BTreeMap::from([(12, 6)])));
        assert!(!loot_matches(&allowed, &BTreeMap::from([(13, 2)])));
    }
}
