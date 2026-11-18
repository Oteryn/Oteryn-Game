//! The native STEP path admits one player, then the Channel timer expels that player.
use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks::{local, world_with_cells};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_gameplay_quests::quest::world_callbacks::{EncounterPlayer, EncounterTiming, Position};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use std::sync::Arc;

const ENTRY: Position = Position {
    x: 100,
    y: 100,
    floor: -7,
};
const INSIDE: Position = Position {
    x: 116,
    y: 117,
    floor: -7,
};
const EXIT: Position = Position {
    x: 98,
    y: 101,
    floor: -7,
};
const KEY: &str = "oteryn:quest-world/m01c-stampede-entry";

#[test]
fn hidden_threats_room_step_admits_one_player_and_expires_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("hidden-threats-room", || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "hidden_threats_room", true, false)
                .await?;
            prepare_current_session(&h).await?;
            // Retain an installed, durable STEP transition while adapting its map
            // effects to the solo room. Quest bindings ship in the content lane.
            let loaded =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            let mut callback = loaded
                .catalogue()
                .authored_world_callbacks()
                .and_then(|callbacks| callbacks.iter().find(|callback| callback.key == KEY))
                .ok_or("authored trial")?
                .clone();
            callback.selector.position = Some(ENTRY);
            callback.selector.appearance = Some(100);
            callback.return_on_failure = true;
            callback.failure_teleport = Some(EXIT);
            let encounter = callback
                .effects
                .encounter
                .as_mut()
                .ok_or("trial encounter")?;
            encounter.players = vec![EncounterPlayer {
                from: ENTRY,
                to: INSIDE,
            }];
            encounter.room_min = Position {
                x: 114,
                y: 114,
                ..INSIDE
            };
            encounter.room_max = Position {
                x: 120,
                y: 120,
                ..INSIDE
            };
            encounter.monsters.truncate(4);
            for (monster, (x, y, creature)) in encounter.monsters.iter_mut().zip([
                (118, 117, "oteryn:creature.cave_spider"),
                (116, 115, "oteryn:creature.tarantula"),
                (116, 116, "oteryn:creature.tarantula"),
                (116, 118, "oteryn:creature.tarantula"),
            ]) {
                monster.position = Position { x, y, ..INSIDE };
                monster.creature = creature.into();
            }
            encounter.timing = Some(EncounterTiming {
                spawn_after_ms: 0,
                reset_after_ms: 300_000,
            });
            let world = world_with_cells(&callback, ENTRY, &[EXIT])?;
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
                        x: 99,
                        y: 100,
                        floor: 7,
                    },
                )
                .ok_or("bundle room")?;
            let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                frame,
                world.map_revision_digest(),
                (99, 100, 7),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                64,
                pin,
            )
            .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: 99,
                        y: 100,
                        floor: 7,
                    },
                )
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
            owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
            owner.quest_catalogue = Some(Arc::new(loaded.catalogue().clone()));
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback.clone()])
                    .is_ok()
            );
            let mut admitted = deps.admitted;
            admitted.runtime_actor = Some(deps.actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            admitted.item_fence = Some(fence.item_fence());
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
            // Replacing the fixture runtime does not transfer the admitted Character lease.
            owner
                .runtime
                .lock()
                .await
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            use crate::gameplay_transport::spell_premium_coordinator::{
                SpellPremiumCoordinator, purchase_fixture,
            };
            let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
            owner.premium_coordinator = Some(SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            ));
            owner.admit_premium(id(40), admitted.game_session_id);
            owner.admit_quest_session(&admitted).await;
            let (other, second) = super::grizzly_boss::vacancy::second_player(
                &owner,
                &deps,
                &h,
                local(Position {
                    x: 100,
                    y: 99,
                    ..ENTRY
                }),
            )
            .await?;

            let _ = owner
                .paced_step(deps.actor, admitted.game_session_id, StepDirection::East)
                .await;
            assert_eq!(
                position(&owner, deps.actor).await?,
                local(INSIDE),
                "STEP commits the solo transfer"
            );
            let _ = owner
                .paced_step(other, second.game_session_id, StepDirection::South)
                .await;
            assert_eq!(position(&owner, other).await?, local(EXIT));
            assert_eq!(position(&owner, deps.actor).await?, local(INSIDE));
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .current_live_creature_census()
                    .map_err(debug)?
                    .len(),
                4
            );
            let receipts: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
            )
            .bind(KEY)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(receipts, 1);

            let due = owner
                .states
                .quest_callbacks
                .next_encounter_due_us()
                .ok_or("room deadline")?;
            owner.drain_quest_encounter_deadlines().await;
            assert_eq!(position(&owner, deps.actor).await?, local(INSIDE));
            owner.clock_origin = std::time::Instant::now()
                .checked_sub(std::time::Duration::from_micros(due))
                .ok_or("deadline clock")?;
            owner.drain_quest_encounter_deadlines().await;
            assert_eq!(position(&owner, deps.actor).await?, local(EXIT));
            assert_eq!(position(&owner, other).await?, local(EXIT));
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .current_live_creature_census()
                    .map_err(debug)?
                    .is_empty()
            );
            Ok(())
        })
    })
}

async fn position(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position())
}
