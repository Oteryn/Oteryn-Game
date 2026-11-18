//! The shipped Path of Defiances teleporter enters a fenced solo trial.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use std::sync::Arc;

const START: MovementLocalPosition = MovementLocalPosition { x: 32963, y: 32313, floor: 8 };
const ENTRY: MovementLocalPosition = MovementLocalPosition { x: 32958, y: 32309, floor: 8 };
const EXIT: MovementLocalPosition = MovementLocalPosition { x: 32963, y: 32307, floor: 8 };
const SCORPION: &str = "oteryn:creature.furious_scorpion";

#[test]
fn seclib2_scorpion_step_admits_ejects_and_persists_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("PostgreSQL test URL is empty".into());
    }
    super::kill_reward_e2e::scenario_thread::run("seclib2-scorpion", || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "seclib2_scorpion", true, false).await?;
            prepare_current_session(&h).await?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) = super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id, "content-1",
            )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native gameplay")?;
            use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
            let world = native_crop(native, NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x128-y126.b3"
                )),
                region: (8, 128, 126),
                bounds: (32943, 32303, 32964, 32315),
                start: (32963, 32313),
                expected: vec![ExpectedTile {
                    position: (32963, 32312),
                    ids: vec![28318, 1949],
                    counted_entry: None,
                }],
            })?;
            let cells = world.movement_cells(activated.qualified_room().movement_cells()).map_err(debug)?;
            let frame = world.frame_binding_digest(deps.admitted.world_id);
            let room = activated.qualified_room().bundle_view(
                cells.clone(), frame, world.map_revision_digest(),
                crate::content::NativeEntryStart { x: START.x, y: START.y, floor: START.floor },
            ).ok_or("bundle combat view")?;
            let pin = ChannelContentPin::from_activation(
                deps.admitted.world_id, activated.activation_sequence(),
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(), frame,
                world.map_revision_digest(), (START.x, START.y, START.floor),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id, deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1, 1, 1, "runtime-scope-assignment:1", 8, pin,
            ).map_err(debug)?;
            let session = deps.admitted.game_session_id;
            let slot = runtime.reserve_fresh_session(session).map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(slot).map_err(debug)?;
            runtime.initialize_pinned_test_position(deps.actor, START).map_err(debug)?;
            native.install_companion_policies(&mut runtime).map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?.catalogue().clone(),
            ));
            let mut admitted = deps.admitted;
            admitted.runtime_actor = Some(deps.actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let fence = owner.current_quest_fence(session).await.map_err(debug)?.ok_or("quest fence")?;
            admitted.item_fence = Some(fence.item_fence());
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(generation.identity().server_artifact_digest().as_slice())
                .bind(generation.identity().client_artifact_digest().as_slice())
                .bind(frame.as_slice()).execute(&h.pool).await?;
            deps.runtime.lock().await.bind_attacker_lease(
                deps.actor, session, crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                ).map_err(debug)?,
            ).map_err(debug)?;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            owner.admit_quest_session(&admitted).await;

            let (step, _) = owner.ordinary_paced_step(deps.actor, session, StepDirection::North, None, None).await;
            assert_eq!(step.disposition, StepDisposition::Moved);
            {
                let runtime = deps.runtime.lock().await;
                assert_eq!(runtime.read_actor_position(deps.actor).map_err(debug)?.position(), ENTRY);
                assert_eq!(runtime.current_live_creature_census().map_err(debug)?.into_iter()
                    .filter(|actor| runtime.matches_live_creature_identity(*actor, SCORPION.as_bytes())).count(), 1);
            }
            let clock: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                .bind(id(41).as_slice())
                .bind("oteryn:quest-progress/quest/u11_80/the_secret_library/darashia/scorpion_timer")
                .fetch_one(&h.pool).await?;
            assert!(clock > 0, "the entry persists its cooldown clock");
            owner.drain_quest_encounter_deadlines().await;
            assert_eq!(deps.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position(), ENTRY);
            owner.clock_origin -= std::time::Duration::from_secs(301);
            owner.drain_quest_encounter_deadlines().await;
            {
                let mut runtime = deps.runtime.lock().await;
                let at = runtime.read_actor_position(deps.actor).map_err(debug)?;
                assert_eq!(at.position(), EXIT);
                assert!(runtime.current_live_creature_census().map_err(debug)?.into_iter()
                    .all(|actor| !runtime.matches_live_creature_identity(actor, SCORPION.as_bytes())));
                let map = owner.bundle_map().ok_or("map")?;
                let closed = crate::map::floor::BundleDoors(&map.facts);
                let proof = crate::map::floor::prepare_quest_callback_teleport(
                    &runtime, &owner.movement_cells, deps.actor, session, at, START, false, &closed,
                ).map_err(debug)?;
                runtime.commit_source_step(proof).map_err(debug)?;
            }
            owner.clock_origin -= std::time::Duration::from_secs(1);
            owner.ordinary_paced_step(deps.actor, session, StepDirection::North, None, None).await;
            assert_eq!(deps.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position(), START);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
