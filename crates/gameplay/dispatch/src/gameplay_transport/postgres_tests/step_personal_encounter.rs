//! A framed STEP, the native lethal publication, and the source owner clock.
use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks_fixtures::{local, wire, world_with_cells};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, CommandId, CommandRef,
    GameSessionId, NodeId, OwnerDamageCommand,
};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::kill_reward::{KillSink, record_creature_kill};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{self, Position, Trigger},
};
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};
use serde_json::json;

const SOURCE: &str = "oteryn:quest-callback/step-personal-encounter-test";
const RAT: &str = "oteryn:creature.rat";
const COUNT: &str = "oteryn:quest-progress/authored-kills/count";
const ORIGIN: Position = position(99, 99);
const ENTRY: Position = position(100, 99);
const LANDING: Position = position(102, 99);
const BOSS: Position = position(104, 99);
const ADD: Position = position(104, 100);
const FOREIGN: Position = position(103, 99);
const NEIGHBOUR: Position = position(102, 100);
const BYSTANDER: Position = position(103, 100);
const EXIT: Position = position(106, 99);

const fn position(x: u16, y: u16) -> Position {
    Position { x, y, floor: -7 }
}

#[test]
fn step_personal_encounter_credits_its_exact_spawn_and_ejects_only_the_admitted_character()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin); // The isolated harness resolves the routed administrator itself.
    let started = std::time::Instant::now();
    let result =
        super::kill_reward_e2e_fixtures::scenario_thread::run("step-personal-encounter", scenario);
    eprintln!(
        "POSTGRES_CASE_COMPLETED step_personal_encounter duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn bindings() -> TestResult<Bindings> {
    let mut value: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quests/src/quest/authored_bindings/fixtures/kill.json"
    )))?;
    let quest = &mut value["quests"][0];
    let mut callback = world_callbacks::load()?
        .into_iter()
        .find(|row| row.key == "oteryn:quest-callback/behemoth")
        .ok_or("callback template")?;
    callback.key = SOURCE.into();
    callback.quest = quest["quest"].as_str().ok_or("quest")?.into();
    callback.trigger = Trigger::StepIn;
    callback.selector.position = Some(ENTRY);
    callback.selector.appearance = Some(100);
    callback.selector.unique = None;
    callback.selector.action = None;
    callback.selector.native = None;
    callback.lever = None;
    callback.teleport = None;
    callback.passages.clear();
    callback.requires_world_admission = true;
    callback.storage = serde_json::from_value(json!([{
        "track":"oteryn:quest-progress/authored-kills/phase","minimum":0,"maximum":0,"set":4
    }]))?;
    let mut callback = serde_json::to_value(callback)?;
    callback["effects"]["encounter"] = json!({
        "minimum_level":1,
        "players":[{"from":ENTRY,"to":LANDING}],
        "monsters":[{"position":BOSS,"creature":RAT},{"position":ADD,"creature":RAT}],
        "room_min":LANDING,"room_max":ADD,
        "personal_exit":{"after_ms":600000,"destination":EXIT}
    });
    let stages = quest["stages"].as_array_mut().ok_or("stages")?;
    stages[0] =
        json!({"key":"entry","completes":false,"trigger":{"kind":"step_in","callback":callback}});
    stages[1]["completes"] = json!(false);
    let counter = stages[1]["trigger"]["counter"]
        .as_object_mut()
        .ok_or("counter")?;
    counter.remove("advance_gate_to");
    counter.insert("maximum".into(), json!(2));
    counter.insert("encounter_source".into(), json!(SOURCE));
    Ok(Bindings::parse(&serde_json::to_vec(&value)?)?)
}

fn scenario() -> TestResult {
    let bindings = bindings()?;
    let catalogue =
        bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?;
    let callback = bindings.world_callbacks().next().ok_or("entry callback")?;
    let world = world_with_cells(
        callback,
        ENTRY,
        &[
            ORIGIN, LANDING, BOSS, ADD, FOREIGN, NEIGHBOUR, BYSTANDER, EXIT,
        ],
    )?;
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "quest_world_callbacks", false, false)
            .await?;
        let result: TestResult = async {
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                    deps.admitted.world_id, "content-1"
                )?;
            let generation = controller.active().ok_or("generation")?;
            let native = generation.native_gameplay().ok_or("native")?;
            let cells = world.movement_cells(activated.qualified_room().movement_cells()).map_err(debug)?;
            let frame = activated.frame_binding().digest();
            let map_digest = world.map_revision_digest();
            let start = crate::content::NativeEntryStart { x: 99, y: 99, floor: 7 };
            let room = activated.qualified_room().bundle_view(cells.clone(), frame, map_digest, start)
                .ok_or("bundle room")?;
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id, 1, generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(), frame, map_digest,
                (start.x, start.y, start.floor),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id, deps.admitted.channel_id, NodeId::decode(&id(1)).map_err(debug)?,
                1, 1, 1, "runtime-scope-assignment:1", 8, pin,
            ).map_err(debug)?;
            let mut admitted = deps.admitted;
            let session = admitted.game_session_id;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime.initialize_pinned_test_position(actor, local(ORIGIN)).map_err(debug)?;
            deps.actor = actor;
            admitted.runtime_actor = Some(actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
            owner.quest_catalogue = Some(std::sync::Arc::new(catalogue));
            let fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fence")?;
            let lease = CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            ).map_err(debug)?;
            owner.runtime.lock().await.bind_attacker_lease(actor, session, lease).map_err(debug)?;
            admitted.item_fence = Some(fence.item_fence());
            let base = owner.bundle_map().ok_or("map")?.overlay.base();
            super::boss_activity_fixtures::issue_content_and_reset_with_runtime_pin(
                &h, &pin, &pin, &crate::map::overlay::map_revision(base)
            ).await?;
            let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
            h.root.initialize_admission_equipment(
                &authority, &h.node, &fence.item_fence(),
                generation.identity().server_artifact_digest(), fresh.operation()
            ).await.map_err(debug)?;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            owner.refresh_quest_session(session, true).await;
            assert!(owner.refresh_source_party_presence(actor, session).await);

            owner.clock_origin -= std::time::Duration::from_secs(2);
            let command = admitted.continuity.next_command_id;
            let reply = wire(&owner, &mut admitted, command, 1, encode_step_intent(StepDirection::East)).await?;
            assert_eq!(decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
            let (boss, add, foreign, neighbour, bystander) = {
                let mut runtime = owner.runtime.lock().await;
                assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), local(LANDING));
                let creatures = runtime.current_live_creature_census().map_err(debug)?;
                assert_eq!(creatures.len(), 2);
                let at = |position| creatures.iter().copied().find(|creature|
                    runtime.read_actor_position(*creature).is_ok_and(|p| p.position() == local(position)));
                let boss = at(BOSS).ok_or("boss")?;
                let add = at(ADD).ok_or("add")?;
                let foreign = runtime.admit_encounter_actor(RAT, 20, local(FOREIGN)).map_err(debug)?;
                runtime.install_creature_policy(foreign, RAT).map_err(debug)?;
                let neighbour = runtime.admit_encounter_actor(RAT, 20, local(NEIGHBOUR)).map_err(debug)?;
                let other = GameSessionId::decode(&id(83)).map_err(debug)?;
                let reservation = runtime.reserve_fresh_session(other).map_err(debug)?;
                let bystander = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime.initialize_pinned_test_position(bystander, local(BYSTANDER)).map_err(debug)?;
                (boss, add, foreign, neighbour, bystander)
            };
            let due = owner.states.quest_callbacks.next_encounter_due_us().ok_or("exit deadline")?;
            owner.drain_quest_encounter_deadlines().await;
            assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), local(LANDING));

            let queue = owner.states.attack.lock().await.kills().clone();
            for (target, command, expected) in [(foreign, command + 1, 0), (boss, command + 2, 1)] {
                let observed = owner.owner_now();
                let now = observed.get() / 1000;
                {
                    let mut runtime = owner.runtime.lock().await;
                    let health = runtime.companion_snapshot(target).map_err(debug)?.health;
                    runtime.borrow_exact_actor_commit_at(now).commit_damage_for_bound_attacker(
                        target, actor, CommandRef::new(session, CommandId::new(command).map_err(debug)?), 0,
                        OwnerDamageCommand { target: RAT.as_bytes(), occurrence: session.as_bytes(),
                            binding: b"step-personal-encounter-death", damage: health },
                    ).map_err(debug)?;
                    record_creature_kill(
                        &mut runtime,
                        KillSink { prey: None, activity: &[], queue: &queue, rewards: native.reward_table(), charms: None },
                        target,
                        crate::combat::CapturedRewardPrincipal {
                            actor, session, character: lease.character_id(), lease_generation: lease.generation(),
                        },
                        now, Some(observed),
                    );
                    assert!(runtime.retains_corpse_projection(target));
                }
                super::qengine_kill_stories_fixtures::drain_deaths(&owner, &admitted, &[]).await?;
                let count: i64 = sqlx::query_scalar(
                    "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)",
                ).bind(lease.character_id().as_bytes().as_slice()).bind(COUNT).fetch_one(&h.pool).await?;
                assert_eq!(count, expected);
            }

            let blocker = owner.runtime.lock().await
                .admit_encounter_actor(RAT, 20, local(EXIT)).map_err(debug)?;
            owner.clock_origin = clock_at(due)?;
            owner.drain_quest_encounter_deadlines().await;
            let retry = owner.states.quest_callbacks.next_encounter_due_us().ok_or("retry deadline")?;
            assert!(retry > due);
            {
                let mut runtime = owner.runtime.lock().await;
                assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), local(LANDING));
                runtime.remove_encounter_actor(blocker).map_err(debug)?;
            }
            owner.clock_origin = clock_at(retry)?;
            owner.drain_quest_encounter_deadlines().await;
            let runtime = owner.runtime.lock().await;
            assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), local(EXIT));
            assert_eq!(runtime.read_actor_position(bystander).map_err(debug)?.position(), local(BYSTANDER));
            assert!(!runtime.contains_live_creature(add));
            assert!(runtime.contains_live_creature(neighbour));
            assert!(owner.states.quest_callbacks.next_encounter_due_us().is_none());
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}

fn clock_at(deadline: u64) -> TestResult<std::time::Instant> {
    std::time::Instant::now()
        .checked_sub(std::time::Duration::from_micros(deadline))
        .ok_or_else(|| "exit clock".into())
}
