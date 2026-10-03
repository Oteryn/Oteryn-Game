//! Test-only bulk arena for real native schedule preparation and owner lifecycle.
//! This does not wire a live server or claim to execute authored spell effects.
#![allow(
    clippy::expect_used,
    reason = "bounded laboratory assertions must fail loudly"
)]

use crate::ability::RevisionSet;
use crate::ai_think::ThinkOccurrence;
use crate::ai_think::profile_schedule::{
    AttackTarget, MonsterSummonFacts, ObservedSummonCount, ProfileScheduleInput,
    ProfileScheduleState, ScheduleError,
};
use crate::content::{
    ProjectV2AbilityAuthoring, ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData,
    ProjectV2BehaviorAuthoring, ProjectV2DefinitionRef,
};
use crate::foundation::{
    ChannelContentPin, ChannelId, ChannelRuntimeV1, CombatDeathFixture, ExactActorRef,
    GameSessionId, MovementLocalPosition, NodeId, ScopeOwnershipGeneration, WorldId,
};
use oteryn_simulation_determinism::GameplayDecisionRoot;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LabInput {
    format_version: u32,
    stage_path: String,
    stage_sha256: String,
    map: Value,
    #[serde(default)]
    selected_creature_keys: Vec<String>,
    seed: u8,
    ticks: u64,
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn pinned_json(path: &str, expected: &str) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let actual = hex(Sha256::digest(&bytes));
    if actual != expected {
        return Err(format!("source SHA256 mismatch: {path}"));
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn scope(seed: u8) -> (WorldId, ChannelId) {
    let mut bytes = [0; 16];
    bytes[0] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    let world = WorldId::decode(&bytes).expect("valid world fixture");
    bytes[15] = seed;
    (
        world,
        ChannelId::decode(&bytes).expect("valid channel fixture"),
    )
}

fn actor(seed: u8) -> ExactActorRef {
    let (world, channel) = scope(seed);
    ExactActorRef::transport_fixture(world, channel)
}

fn revisions() -> RevisionSet {
    RevisionSet::new(
        "lab-rules-v1",
        "lab-content-v1",
        "lab-map-v1",
        "lab-formula-v1",
        "lab-sim-v1",
    )
    .expect("valid laboratory revisions")
}

fn run_schedule(
    behavior: &ProjectV2BehaviorAuthoring,
    abilities: &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
    seed: u8,
    ticks: u64,
    issuer: ExactActorRef,
    target: ExactActorRef,
) -> Result<Value, String> {
    let revisions = revisions();
    let root = GameplayDecisionRoot::from_bytes([seed; 32]);
    let mut state = ProfileScheduleState::new(issuer);
    let mut counts = [0_usize; 3];
    let mut transcript = Sha256::new();
    for sequence in 0..ticks {
        let occurrence = ThinkOccurrence {
            actor: issuer,
            sequence,
        };
        // Test-owner facts: empty summon population and one adjacent target.
        // These prepare requests; no summoned actor is actually installed.
        let facts = behavior.summons.as_ref().map(|summons| MonsterSummonFacts {
            occurrence,
            is_summon: false,
            target_with_path: (behavior.targeting.hostile && behavior.targeting.can_target)
                .then_some(target),
            total_count: 0,
            entry_counts: summons
                .entries
                .iter()
                .map(|entry| ObservedSummonCount {
                    creature: entry.creature.clone(),
                    count: 0,
                })
                .collect(),
        });
        let input = ProfileScheduleInput {
            occurrence,
            behavior,
            abilities,
            target: Some(AttackTarget {
                actor: target,
                same_floor_distance: Some(1),
            }),
            revisions: &revisions,
            root: &root,
        };
        let plan = state
            .prepare_with_summons(input, facts.as_ref())
            .map_err(|e| format!("{e:?}"))?;
        let replay = state
            .prepare_with_summons(input, facts.as_ref())
            .map_err(|e| format!("retry: {e:?}"))?;
        if replay != plan {
            return Err("retry changed prepared plan".to_owned());
        }
        counts[0] += plan.proposals.len();
        counts[1] += plan.summon_proposals.len();
        counts[2] += plan.melee_delegated;
        transcript.update(format!("{plan:?}").as_bytes());
    }
    Ok(json!({
        "ticks": ticks, "ability_proposals": counts[0], "summon_proposals": counts[1],
        "melee_delegations": counts[2], "retry_idempotent": true,
        "transcript_sha256": hex(transcript.finalize()),
        "effects_executed": false, "summons_installed": false,
        "facts": "same_channel_test_owner_adjacent_target_empty_summon_population",
        "same_channel_owner_issued_actor_references":true,"map_actor_placement_tested":true,
    }))
}

fn arena_actors(
    map: &Value,
    key: &str,
    health: i64,
) -> Result<(ChannelRuntimeV1, ExactActorRef, ExactActorRef), String> {
    let (world, channel) = scope(40);
    let mut node_bytes = [0; 16];
    node_bytes[0] = 1;
    node_bytes[6] = 0x70;
    node_bytes[8] = 0x80;
    node_bytes[15] = 41;
    let node = NodeId::decode(&node_bytes).map_err(|e| format!("{e:?}"))?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        node,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::test(world),
    )
    .map_err(|e| format!("{e:?}"))?;
    node_bytes[15] = 42;
    let session = GameSessionId::decode(&node_bytes).map_err(|e| format!("{e:?}"))?;
    let reservation = runtime
        .reserve_fresh_session(session)
        .map_err(|e| format!("{e:?}"))?;
    let player = runtime
        .commit_fresh_session(reservation)
        .map_err(|e| format!("{e:?}"))?;
    let position = |slot: &Value| -> Result<MovementLocalPosition, String> {
        Ok(MovementLocalPosition {
            x: i32::try_from(slot["x"].as_i64().ok_or("invalid slot x")?)
                .map_err(|e| e.to_string())?,
            y: i32::try_from(slot["y"].as_i64().ok_or("invalid slot y")?)
                .map_err(|e| e.to_string())?,
            floor: i16::try_from(slot["floor"].as_i64().ok_or("invalid slot floor")?)
                .map_err(|e| e.to_string())?,
        })
    };
    runtime
        .initialize_movement_test_position(player, position(&map["player_slot"])?)
        .map_err(|e| format!("{e:?}"))?;
    let creature = runtime
        .admit_monster_lab_creature(position(&map["monster_slot"])?, key, health)
        .map_err(|e| format!("{e:?}"))?;
    if player == creature {
        return Err("arena actor identities collided".to_owned());
    }
    Ok((runtime, player, creature))
}

fn native_owner_lifecycle(health: i64) -> Value {
    let (world, channel) = scope(20);
    let generation = ScopeOwnershipGeneration::new(1).expect("generation");
    let mut fixture = CombatDeathFixture::new_with_health(world, channel, generation, health)
        .expect("owner fixture");
    let damage = health.min(7);
    let first = fixture
        .strike("lab-first", damage)
        .expect("real owner damage commit");
    let retry = fixture
        .strike("lab-first", damage)
        .expect("real owner replay");
    assert_eq!(first.health_before, health);
    assert_eq!(first.health_after, health - damage);
    assert_eq!(retry.health_after, health - damage);
    if first.health_after > 0 {
        let lethal = fixture
            .strike("lab-lethal", first.health_after)
            .expect("real owner lethal damage");
        assert_eq!(lethal.health_after, 0);
    }
    let first_death = fixture
        .project_death()
        .expect("real Combat death projection");
    let retry_death = fixture.project_death().expect("death projection retry");
    assert_eq!(first_death, retry_death);
    fixture.despawn().expect("real owner despawn");
    assert!(fixture.strike("lab-after-despawn", 1).is_err());
    json!({"status":"PASS", "implementation":"CombatDeathFixture/native_owner_carrier",
        "initial_hp":health,"health_supplied_by_caller":true,"damage_commit":true,
        "retry_idempotent":true,"lethal_death_projection":true,"despawn_refuses_damage":true,
        "fixture_position":{"x":100,"y":100,"floor":7},"map_arena_position_tested":false})
}

fn validate_map(map: &Value) -> Result<(), String> {
    let path = map["source_path"]
        .as_str()
        .ok_or("missing map source_path")?;
    let digest = map["source_sha256"]
        .as_str()
        .ok_or("missing map source_sha256")?;
    let source = pinned_json(path, digest)?;
    let placements = source["placements"]
        .as_array()
        .ok_or("missing source placements")?;
    let cells = source["native_first_entry"]["cells"]
        .as_array()
        .ok_or("missing explicit native cells")?;
    for name in ["player_slot", "monster_slot"] {
        let slot = &map[name];
        let key = slot["key"].as_str().ok_or("missing slot key")?;
        let placement = placements
            .iter()
            .find(|p| p["key"] == key)
            .ok_or("slot missing from pinned source placements")?;
        if slot["walkability"] != "Walkable"
            || slot["spawn_eligible"] != true
            || !cells
                .iter()
                .any(|c| c["placement_key"] == key && c["collision"] == "Walkable")
            || ["x", "y", "floor"]
                .iter()
                .any(|field| slot[*field] != placement[*field])
            || slot["terrain"] != placement["definition"]
        {
            return Err("slot does not match explicitly walkable pinned native cell".to_owned());
        }
    }
    let player = &map["player_slot"];
    let monster = &map["monster_slot"];
    let px = player["x"].as_i64().ok_or("invalid player x")?;
    let py = player["y"].as_i64().ok_or("invalid player y")?;
    let mx = monster["x"].as_i64().ok_or("invalid monster x")?;
    let my = monster["y"].as_i64().ok_or("invalid monster y")?;
    if player["floor"] != monster["floor"] || px.abs_diff(mx).max(py.abs_diff(my)) != 1 {
        return Err("laboratory slots must be distinct adjacent same-floor cells".to_owned());
    }
    Ok(())
}

fn bulk(input: &LabInput) -> Result<Value, String> {
    if input.format_version != 1 || !(1..=600).contains(&input.ticks) {
        return Err("unsupported format or ticks outside 1..600".to_owned());
    }
    validate_map(&input.map)?;
    let stage = pinned_json(&input.stage_path, &input.stage_sha256)?;
    let profiles: Vec<ProjectV2AuthoringProfile> = serde_json::from_value(
        stage
            .get("authoring_profiles")
            .cloned()
            .ok_or("missing authoring_profiles")?,
    )
    .map_err(|e| e.to_string())?;
    let mut abilities = BTreeMap::new();
    let mut behaviors = BTreeMap::new();
    let mut creatures = BTreeMap::new();
    for profile in profiles {
        match profile.data {
            ProjectV2AuthoringProfileData::Ability(data) => {
                abilities.insert(profile.target, data);
            }
            ProjectV2AuthoringProfileData::Behavior(data) => {
                behaviors.insert(profile.target, data);
            }
            ProjectV2AuthoringProfileData::Creature(data) => {
                creatures.insert(profile.target, data);
            }
            _ => {}
        }
    }
    let selected: BTreeSet<_> = input.selected_creature_keys.iter().collect();
    let mut outcomes = Vec::new();
    let mut seen = BTreeSet::new();
    let records = stage
        .get("records")
        .and_then(Value::as_array)
        .ok_or("missing records")?;
    for record in records.iter().filter(|r| r["kind"] == "Creature") {
        let identity: ProjectV2DefinitionRef =
            serde_json::from_value(record["identity"].clone()).map_err(|e| e.to_string())?;
        if !selected.is_empty() && !selected.contains(&identity.key) {
            continue;
        }
        if !seen.insert(identity.key.clone()) {
            return Err("duplicate Creature identity".to_owned());
        }
        let result = (|| {
            let profile = creatures
                .get(&identity)
                .ok_or("missing exact Creature profile")?;
            let health = i64::try_from(profile.health.ok_or("missing authored HP")?)
                .map_err(|e| e.to_string())?;
            let (_runtime, player, creature) = arena_actors(&input.map, &identity.key, health)?;
            let behavior_ref: ProjectV2DefinitionRef =
                serde_json::from_value(record["behavior"].clone()).map_err(|e| e.to_string())?;
            let behavior = behaviors
                .get(&behavior_ref)
                .ok_or("missing exact Behavior profile")?;
            let first = run_schedule(
                behavior,
                &abilities,
                input.seed,
                input.ticks,
                creature,
                player,
            )?;
            if first
                != run_schedule(
                    behavior,
                    &abilities,
                    input.seed,
                    input.ticks,
                    creature,
                    player,
                )?
            {
                return Err("fresh-state deterministic replay changed".to_owned());
            }
            let lifecycle = native_owner_lifecycle(health);
            Ok((first, lifecycle, health))
        })();
        outcomes.push(match result {
            Ok((schedule, lifecycle, health)) => {
                json!({"key":identity.key,"status":"PASS_NATIVE_SCHEDULE_PREPARATION",
                "schedule":schedule,"native_owner_lifecycle":lifecycle,
                "authored_hp_tested":health,"authored_source_key_admitted":true,
                "fresh_replay_identical":true,"live_gameplay_tested":false})
            }
            Err(error) => json!({"key":identity.key,"status":"COMPONENT_BLOCKED",
                "error":error,"live_gameplay_tested":false}),
        });
    }
    if selected.iter().any(|key| !seen.contains(*key)) {
        return Err("selected Creature does not exist in native stage".to_owned());
    }
    if outcomes.is_empty() {
        return Err("no Creature selected".to_owned());
    }
    let passed = outcomes
        .iter()
        .filter(|v| v["status"] == "PASS_NATIVE_SCHEDULE_PREPARATION")
        .count();
    Ok(
        json!({"format_version":1,"mode":"headless_native_component_arena",
        "scope":"headless_native_component_arena","passed":passed==outcomes.len(),
        "map_source_sha256":input.map["source_sha256"],
        "stage_sha256":input.stage_sha256,"map":input.map,"seed":input.seed,
        "creatures_tested":outcomes.len(),"passed_profiles":passed,"blocked":outcomes.len()-passed,
        "live_server_started":false,"spell_effect_execution_tested":false,
        "native_owner_lifecycle":native_owner_lifecycle(20),"outcomes":outcomes}),
    )
}

#[test]
fn run_from_manifest() {
    let Ok(path) = std::env::var("OTERYN_MONSTER_LAB_INPUT") else {
        eprintln!("monster_lab: no external batch input; focused scenario tests still run");
        return;
    };
    let bytes = std::fs::read(path).expect("read bounded lab input");
    assert!(bytes.len() <= 1024 * 1024, "lab manifest exceeds 1MiB");
    let input: LabInput = serde_json::from_slice(&bytes).expect("closed lab input");
    let output = bulk(&input).expect("run native component laboratory");
    let path = std::env::var("OTERYN_MONSTER_LAB_OUTPUT").expect("explicit output path");
    let parent = Path::new(&path).parent().expect("output parent");
    std::fs::create_dir_all(parent).expect("output directory");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&output).expect("encode outcomes"),
    )
    .expect("write laboratory outcomes");
    eprintln!(
        "monster_lab: tested={} passed={} blocked={} output={path}",
        output["creatures_tested"], output["passed_profiles"], output["blocked"]
    );
}

#[test]
fn monster_lab_real_owner_lifecycle_and_replay() {
    for health in [1, 20, 1_000_000_000] {
        assert_eq!(native_owner_lifecycle(health)["status"], "PASS");
    }
}

#[test]
fn monster_lab_refuses_zero_and_negative_hp() {
    let (world, channel) = scope(20);
    let generation = ScopeOwnershipGeneration::new(1).expect("generation");
    for health in [0, -1] {
        assert!(CombatDeathFixture::new_with_health(world, channel, generation, health).is_err());
    }
}

#[test]
fn monster_lab_rejects_invalid_format_before_stage_access() {
    let input = LabInput {
        format_version: 2,
        stage_path: "absent".to_owned(),
        stage_sha256: "absent".to_owned(),
        map: json!({}),
        selected_creature_keys: vec![],
        seed: 1,
        ticks: 1,
    };
    assert!(bulk(&input).is_err());
}

#[test]
fn monster_lab_refuses_wrong_source_hash() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/content/project/native_entry_room.json"
    );
    assert!(pinned_json(path, &"0".repeat(64)).is_err());
}

#[test]
fn monster_lab_refuses_blocked_spawn_slot() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/content/project/native_entry_room.json"
    );
    let bytes = std::fs::read(path).expect("native map source");
    let digest = hex(Sha256::digest(&bytes));
    let source: Value = serde_json::from_slice(&bytes).expect("native map JSON");
    let placement = &source["placements"][2];
    let slot = json!({"key":placement["key"], "x":placement["x"],
        "y":placement["y"],"floor":placement["floor"],"terrain":placement["definition"],
        "walkability":"Walkable","spawn_eligible":true});
    let map = json!({"source_path":path,"source_sha256":digest,
        "player_slot":slot,"monster_slot":slot});
    assert!(validate_map(&map).is_err());
}

#[test]
fn monster_lab_actor_mismatch_refuses_native_schedule() {
    let behavior: ProjectV2BehaviorAuthoring = serde_json::from_value(json!({
        "movement":{"can_walk":true,"pass_through":false,"pushable":false,
            "push_items":false,"push_creatures":false,"walks_on_energy":false,
            "walks_on_fire":false,"walks_on_poison":false},
        "targeting":{"hostile":true,"can_target":true,"sense_invisible":false,
            "target_distance_tiles":1,"static_attack_chance_ppm":1000000,"flee_health":0}
    }))
    .expect("native behavior");
    let mut state = ProfileScheduleState::new(actor(1));
    assert_eq!(
        state.prepare(
            ThinkOccurrence {
                actor: actor(2),
                sequence: 0
            },
            &behavior,
            &BTreeMap::new(),
            None,
            &revisions(),
            &GameplayDecisionRoot::from_bytes([1; 32])
        ),
        Err(ScheduleError::ActorMismatch)
    );
}
