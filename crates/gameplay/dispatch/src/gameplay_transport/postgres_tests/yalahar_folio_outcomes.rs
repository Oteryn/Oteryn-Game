//! Crystal 201189f8 actions_mission09.lua:7-49.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{local, wire};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
    encode_use_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.fixture_yalahar_folio";
const MISSION: &str = "oteryn:quest-progress/fixture-yalahar/mission09";
const COUNT: &str = "oteryn:quest-progress/fixture-yalahar/read-count";
const CLOCK: &str = "oteryn:quest-progress/fixture-yalahar/headache";
const GROUP: &str = "oteryn:quest-random-use/fixture-yalahar/folio";
const BOOKWORM: &str = "oteryn:creature.enraged_bookworm";
// Crystal uses a 1..=100 first roll and, for the headache, a 1..=100 second roll.
const DONOR_OUTCOMES: [(&str, u16, Option<&str>, u16); 6] = [
    ("headache", 1520, None, 32),
    ("headache-bookworm", 190, Some(BOOKWORM), 32),
    (
        "headache-fire-devil",
        95,
        Some("oteryn:creature.fire_devil"),
        32,
    ),
    (
        "headache-war-golem",
        95,
        Some("oteryn:creature.war_golem"),
        32,
    ),
    ("relocate", 100, None, 11),
    ("success", 8000, None, 30),
];
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/shadows_of_yalahar_quest/actions_mission09.lua:7-49";
const FOLIO: Position = Position {
    x: 32797,
    y: 31197,
    floor: -9,
};
const LANDING: Position = Position {
    x: 33324,
    y: 32173,
    floor: -6,
};
const START: Position = Position {
    x: FOLIO.x - 1,
    ..FOLIO
};

#[test]
fn folio_use_relocates_and_resets_success_without_starting_a_headache() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        exercise(admin, "relocate", false).await
    })
}

#[test]
fn folio_use_starts_a_headache_and_spawns_one_active_bookworm() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        exercise(admin, "headache-bookworm", false).await
    })
}

#[test]
fn folio_success_counts_the_first_read_from_a_negative_legacy_value() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        exercise(admin, "success", false).await
    })
}

#[test]
fn folio_reconnect_projects_the_committed_headache_spawn_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        exercise(admin, "headache-bookworm", true).await
    })
}

fn catalogue(initial_count: i64) -> TestResult<QuestStateCatalogue> {
    let routes: Vec<_> = DONOR_OUTCOMES.into_iter().map(|(suffix, weight, spawn, effect)| {
        let relocation = suffix == "relocate";
        let success = suffix == "success";
        let headache = !relocation && !success;
        json!({
            "key": format!("oteryn:quest-stage/fixture-yalahar/folio-{suffix}"), "completes": false,
            "trigger": {"kind": "map_use", "callback": {
                "key": format!("oteryn:quest-callback/fixture-yalahar/folio-{suffix}"),
                "quest": QUEST, "trigger": "use", "reversible": false,
                "selector": {"position": FOLIO, "ordinal": 1, "appearance": 9193},
                "effects": {"random_use": {
                    "group": GROUP, "weight": weight, "spawn_at_user": spawn, "user_effect": effect
                }},
                "teleport": relocation.then_some(LANDING),
                "storage": [{"track": MISSION, "minimum": 1, "maximum": 1, "set": 1}],
                "state_ops": [{
                    "track": COUNT, "from": {"kind": "between", "minimum": -1, "maximum": 7},
                    "effect": if success {
                        json!({"kind": "add_at_least", "value": 1, "minimum": 1})
                    } else {
                        json!({"kind": "add_at_least", "value": -7, "minimum": 0})
                    }
                }],
                "state_operations": [{
                    "track": CLOCK, "from": {"kind": "elapsed_at_least", "seconds": 60},
                    "effect": if headache { json!({"kind": "set_now"}) } else { json!({"kind": "add", "value": 0}) }
                }],
                "requires_world_admission": true, "passages": [], "offerings": [], "return_on_failure": false,
                "sources": [SOURCE]
            }}
        })
    }).collect();
    let data = json!({"schema": 1, "quests": [{
        "quest": QUEST, "recipe_sha256": Sha256::digest(SOURCE).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        "sources": [SOURCE],
        "tracks": [
            {"key": MISSION, "initial": 1, "minimum": 1, "maximum": 2},
            {"key": COUNT, "initial": initial_count, "minimum": -1, "maximum": 8},
            {"key": CLOCK, "initial": -1, "minimum": -1, "maximum": i64::MAX}
        ],
        "stages": routes
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&data)?)?.attach(&base)?)
}

async fn exercise(admin: String, branch: &str, interrupt_projection: bool) -> TestResult {
    let success = branch == "success";
    let expect_headache = branch == "headache-bookworm";
    let h =
        Harness::create_with_admission_seed(admin, "yalahar_folio_outcomes", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("content generation")?;
    let catalogue = catalogue(if success { -1 } else { 4 })?;
    let relocation = catalogue
        .authored_bindings()
        .ok_or("folio bindings")?
        .world_callbacks()
        .find(|callback| callback.teleport == Some(LANDING))
        .ok_or("folio relocation branch")?;
    let world = super::quest_world_callbacks::world(relocation, FOLIO)?;
    super::arena_match::fixture::runtime_at(&mut deps, generation, &pin, &world, local(START))?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue));
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("map USE capabilities")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest session")?;
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(world.frame_binding_digest(admitted.world_id).as_slice()).execute(&h.pool).await?;
    owner.admit_quest_session(&admitted).await;
    let quest_now = 1_800_000_000;
    h.root.set_test_quest_clock(quest_now).map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let tile = crate::map::overlay::TilePos {
        x: FOLIO.x,
        y: FOLIO.y,
        floor: FOLIO.floor,
    };
    let key = crate::map::view::placement_key(tile, 1).ok_or("folio")?;
    let request = || {
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })
        .map_err(debug)
    };
    let req = request()?;
    // Pick an occurrence in the authored interval. Production dispatch still
    // selects, validates, commits and projects that interval itself.
    let session = admitted.game_session_id;
    let command_for = |branch: &str, start| {
        let mut first_roll = 0u64;
        let mut wanted = None;
        for (key, weight, _, _) in DONOR_OUTCOMES {
            let next = first_roll + u64::from(weight);
            if key == branch {
                wanted = Some(first_roll..next);
            }
            first_roll = next;
        }
        assert_eq!(first_roll, 10_000);
        let wanted = wanted?;
        (start..start + 8_192).find(|number: &u64| {
            let digest = Sha256::new()
                .chain_update(b"oteryn:quest-random-use-v1")
                .chain_update(session.as_bytes())
                .chain_update(deps.actor.placement_identity())
                .chain_update(number.to_be_bytes())
                .chain_update(key.to_be_bytes())
                .chain_update(GROUP.as_bytes())
                .finalize();
            let mut prefix = [0; 8];
            prefix.copy_from_slice(&digest[..8]);
            wanted.contains(&(u64::from_be_bytes(prefix) % first_roll))
        })
    };
    let command = command_for(branch, 1).ok_or("folio branch occurrence")?;
    admitted.continuity.next_command_id = command;
    if interrupt_projection {
        // A lost result after the durable commit must resume the same branch
        // before dispatching any subsequent command through the Channel.
        owner
            .states
            .quest_callbacks
            .interrupt_after_commit
            .store(true, std::sync::atomic::Ordering::Release);
        assert!(
            wire(
                &owner,
                &mut admitted,
                command,
                COMMAND_TYPE_USE_INTENT,
                req.clone()
            )
            .await
            .is_err(),
            "the interrupted USE closes without acknowledging the outcome"
        );
        assert!(owner.has_pending_quest_callback(session));
        let saved = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(saved.tracks().get(COUNT), Some(&0));
        assert_eq!(saved.tracks().get(CLOCK), Some(&quest_now));
        let runtime = owner.runtime.lock().await;
        assert!(
            !runtime
                .current_live_creature_census()
                .map_err(debug)?
                .into_iter()
                .any(|actor| runtime.matches_live_creature_identity(actor, BOOKWORM.as_bytes()))
        );
    }
    let first = wire(
        &owner,
        &mut admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        req.clone(),
    )
    .await?;
    assert_eq!(
        decode_use_result(&first).map_err(debug)?,
        UseDisposition::Committed
    );
    assert!(!owner.has_pending_quest_callback(session));
    let snapshot = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(snapshot.tracks().get(COUNT), Some(&i64::from(success)));
    if expect_headache {
        assert_eq!(snapshot.tracks().get(CLOCK), Some(&quest_now));
    } else {
        assert_eq!(snapshot.tracks().get(CLOCK).copied().unwrap_or(-1), -1);
    }
    assert_eq!(
        wire(&owner, &mut admitted, command, COMMAND_TYPE_USE_INTENT, req).await?,
        first
    );
    {
        let runtime = owner.runtime.lock().await;
        let at = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        let spawned: Vec<_> = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .filter(|actor| runtime.matches_live_creature_identity(*actor, BOOKWORM.as_bytes()))
            .collect();
        if expect_headache {
            assert_eq!(at, local(START));
            assert_eq!(
                spawned.len(),
                1,
                "replaying USE cannot spawn another bookworm"
            );
            let creature = runtime.companion_snapshot(spawned[0]).map_err(debug)?;
            assert_eq!(creature.position, at);
            assert_eq!(creature.state.policy.definition_key, BOOKWORM);
            assert_eq!(creature.health, creature.maximum_health);
        } else {
            assert_eq!(at, local(if success { START } else { LANDING }));
            assert!(spawned.is_empty());
        }
    }
    if expect_headache {
        // Crystal actions_mission09.lua:20-24 permits reading at equality.
        for (advance, disposition, count) in [
            (59, UseDisposition::Rejected, 0),
            (1, UseDisposition::Committed, 1),
        ] {
            h.root.advance_test_quest_clock(advance).map_err(debug)?;
            let retry = command_for("success", admitted.continuity.next_command_id)
                .ok_or("folio retry branch occurrence")?;
            admitted.continuity.next_command_id = retry;
            let reply = wire(
                &owner,
                &mut admitted,
                retry,
                COMMAND_TYPE_USE_INTENT,
                request()?,
            )
            .await?;
            assert_eq!(decode_use_result(&reply).map_err(debug)?, disposition);
            let snapshot = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(snapshot.tracks().get(COUNT), Some(&count));
            assert_eq!(snapshot.tracks().get(CLOCK), Some(&quest_now));
        }
    }
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(receipts, if expect_headache { 2 } else { 1 });
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
