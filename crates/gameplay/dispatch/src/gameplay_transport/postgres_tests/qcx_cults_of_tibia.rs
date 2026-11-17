use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks::{local, wire, world};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, NodeId};
use crate::gameplay_transport::SeamRooms;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::{Position, load};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

#[test]
fn cult_teleports_publish_the_source_effect_at_the_landing() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("qcx-cults", || {
        run(async |admin| {
            let started = std::time::Instant::now();
            let callbacks = load()?;
            let mut mismatches = Vec::new();
            for (name, x, y, floor) in [
                ("carlin_exit", 32403, 31813, -8),
                ("essence_exit", 32349, 31668, -10),
                ("orc_entry", 33182, 31837, -15),
                ("orc_exit", 33168, 31930, -12),
            ] {
                let case_started = std::time::Instant::now();
                let key = format!("oteryn:quest-callback/cults_of_tibia/{name}");
                let callback = callbacks.iter().find(|entry| entry.key == key).ok_or(key)?;
                let trigger = callback.selector.position.ok_or("source trigger")?;
                let destination = Position { x, y, floor };
                let origin = local(Position {
                    x: trigger.x - 1,
                    ..trigger
                });
                let world = world(callback, trigger)?;
                reset();
                let h = Harness::create_with_admission_seed(
                    admin,
                    &format!("qcx_cults_{name}"),
                    true,
                    false,
                )
                .await?;
                prepare_current_session(&h).await?;
                let mut deps = Dependencies::new()?;
                let (controller, activated) =
                    super::death_ml_loss_composed::production_generation_with_pin(
                        deps.admitted.world_id,
                        "content-1",
                    )?;
                let generation = controller.active().ok_or("active generation")?;
                let cells = world
                    .movement_cells(activated.qualified_room().movement_cells())
                    .map_err(debug)?;
                let frame = world.frame_binding_digest(deps.admitted.world_id);
                let room = activated
                    .qualified_room()
                    .bundle_view(
                        cells.clone(),
                        frame,
                        world.map_revision_digest(),
                        crate::content::NativeEntryStart {
                            x: origin.x,
                            y: origin.y,
                            floor: origin.floor,
                        },
                    )
                    .ok_or("bundle room")?;
                let pin = ChannelContentPin::from_activation(
                    deps.admitted.world_id,
                    1,
                    generation.identity().server_artifact_digest(),
                    generation.identity().client_artifact_digest(),
                    frame,
                    world.map_revision_digest(),
                    (origin.x, origin.y, origin.floor),
                );
                let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                    deps.admitted.world_id,
                    deps.admitted.channel_id,
                    NodeId::decode(&id(1)).map_err(debug)?,
                    1,
                    1,
                    1,
                    "runtime-scope-assignment:1",
                    8,
                    pin,
                )
                .map_err(debug)?;
                let reservation = runtime
                    .reserve_fresh_session(deps.admitted.game_session_id)
                    .map_err(debug)?;
                deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .initialize_pinned_test_position(deps.actor, origin)
                    .map_err(debug)?;
                deps.runtime = tokio::sync::Mutex::new(runtime);

                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(generation);
                owner.movement_cells = (&cells).into();
                owner.rooms = SeamRooms::new(None, Some(&room));
                let mut admitted = deps.admitted;
                admitted.runtime_actor = Some(deps.actor);
                admitted.first_entry = FirstEntryOutcome::Positioned;
                let fence = owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(|()| "quest fence")?
                    .ok_or("quest fence")?;
                owner
                    .runtime
                    .lock()
                    .await
                    .bind_attacker_lease(
                        deps.actor,
                        admitted.game_session_id,
                        CharacterLease::new(
                            CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                            fence.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                admitted.item_fence = Some(fence.item_fence());
                deps.initialize(&owner).await;
                let reply = wire(
                    &owner,
                    &mut admitted,
                    1,
                    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                    encode_step_intent(StepDirection::East),
                )
                .await?;
                assert_eq!(
                    decode_step_result(&reply).map_err(debug)?,
                    StepDisposition::Moved
                );
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    local(destination),
                    "{name}"
                );
                let map = owner.bundle_map().ok_or("callback map")?;
                let effects = map.facts.movement_effects_since(0);
                let observed: Vec<_> = effects
                    .iter()
                    .map(|&(_, at, effect)| (at, effect))
                    .collect();
                if observed != [(TilePos { x, y, floor }, 11)] {
                    mismatches.push((name, observed));
                }
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await?;
                eprintln!(
                    "QCX_CULTS_POSTGRES_CASE {name} duration={:?}",
                    case_started.elapsed()
                );
            }
            eprintln!(
                "QCX_CULTS_POSTGRES_COMPLETED duration={:?}",
                started.elapsed()
            );
            assert!(
                mismatches.is_empty(),
                "unexpected teleport effects: {mismatches:?}"
            );
            Ok(())
        })
    })
}
