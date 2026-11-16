//! An accepted Spite Instance keeps its original entry pins after the source
//! Channel activates new content: a real Player STEP onto a real private
//! Global fire still consumes it once through the Instance authority arm.
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use super::{TestResult, reset};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::durability::creature_source_items::SpiteGlobalFireCallback;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use oteryn_protocol_oteryn::boss_encounter as wire;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

#[test]
fn spite_step_instance_survives_channel_activation_advance() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("spite-step-instance-activation", || {
        run(async |admin| {
            use oteryn_gameplay_quests::quest::soul_war::source_items as rule;
            reset();
            let source = PinnedActivitySourceRoom::GoshnarsSpite;
            // The real Spite lever is the source Player's start position.
            let lever = {
                let probe = SourceScene::new(
                    source,
                    MovementLocalPosition {
                        x: 33774,
                        y: 31634,
                        floor: 14,
                    },
                )?;
                let template = probe
                    .controller
                    .active()
                    .ok_or("active Spite generation")?
                    .qualify_soul_war_activity_template(&probe.room, "content-1", source)
                    .map_err(|error| format!("actual Spite source template: {error:?}"))?;
                *template.lever_positions().first().ok_or("Spite lever")?
            };
            let scene = SourceScene::new(
                source,
                MovementLocalPosition {
                    x: lever.x,
                    y: lever.y,
                    floor: i16::try_from(lever.z)?,
                },
            )?;
            let template = scene
                .controller
                .active()
                .ok_or("active Spite generation")?
                .qualify_soul_war_activity_template(&scene.room, "content-1", source)
                .map_err(|error| format!("actual Spite source template: {error:?}"))?;
            assert_eq!(template.minimum_level(), 250);
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(
                admin,
                "spite_step_instance_activation",
                false,
                false,
            )
            .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, source)?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[
                    4,
                    6,
                    7,
                    10,
                    11,
                    15,
                    18,
                    23,
                    92,
                    wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                ],
            )
            .ok_or("native Spite capabilities")?;
            prepare_player(&h, &owner, &mut admitted, 250).await?;
            let current = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|_| "current task fence")?
                .ok_or("current task Character")?;
            let mut slot = owner.revision_sequencer.acquire(current.character_id).await;
            let started = owner
                .root
                .commit_soul_war_command(
                    owner.character,
                    owner.holder,
                    current,
                    CommandRef::new(admitted.game_session_id, CommandId::new(1).map_err(debug)?),
                    crate::durability::quest_state::soul_war::SoulWarCommand::Start,
                )
                .await?;
            assert_eq!(started.result, Ok(()));
            slot.observe_endgame_progression();
            drop(slot);
            admitted.item_fence = crate::gameplay_transport::item_fence_of(
                FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0,
            );
            owner
                .activity_runtimes
                .remember(admitted)
                .map_err(|_| "current source connection")?;
            ActivityHandoffOwner::new(h.root.clone())
                .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
                .await?;
            owner
                .activity_runtimes
                .register_template(&template)
                .map_err(|_| "source template")?;
            let destination = super::activity_runtime::allocate_ready_instance(&h, 233).await?;
            assert!(matches!(destination, RuntimeScopeRefV1::Instance { .. }));
            let (_, reply, barrier) = super::boss_activity::exchange_capture(
                &owner,
                &mut admitted,
                template.boss_key(),
                true,
                &mut Vec::new(),
            )
            .await?;
            assert!(
                matches!(reply, wire::BossActivityReply::Accepted { .. }),
                "{reply:?}"
            );
            assert!(barrier.is_some(), "actual native scope barrier");
            let fence = admitted.item_fence.ok_or("actual destination fence")?;
            assert_eq!(fence.runtime_scope, destination);
            let registered = owner
                .activity_runtimes
                .get(destination, fence.scope_ownership_generation.get())
                .ok_or("actual destination owner")?;
            let actor = admitted.runtime_actor.ok_or("actual transferred Player")?;

            let mut view = owner.runtime_view(&registered);

            // Place the transferred Player one row south of the first field
            // through the existing quest-encounter movement owner.
            move_player(
                &view,
                actor,
                admitted.game_session_id,
                MovementLocalPosition {
                    x: 33743,
                    y: 31629,
                    floor: 14,
                },
            )
            .await?;

            // One real private Global fire in the Instance, through the real
            // fenced native/SQL producer on the Instance's own Item lane.
            h.root.set_test_quest_clock(1_800_000_000).map_err(debug)?;
            let native = scene
                .controller
                .active()
                .and_then(|active| active.native_gameplay())
                .ok_or("actual source generation")?;
            let registration = native
                .qualify_spite_global_event(&scene.room)
                .ok_or("real registered Spite source")?;
            let permit = view
                .spell_lane_permit()
                .await
                .ok_or("physical Instance Item lane")?;
            let (callback, prepared) = {
                let mut runtime = view.runtime.lock().await;
                let callback = SpiteGlobalFireCallback::from_current_registration(
                    &mut runtime,
                    &scene.room,
                    native,
                    &registration,
                    1,
                    0,
                    view.owner_now().get(),
                )
                .map_err(debug)?;
                let prepared = callback.prepare_current_write(&runtime).map_err(debug)?;
                (callback, prepared)
            };
            let created = view
                .root
                .commit_spite_global_fire(
                    &permit,
                    view.character,
                    view.holder,
                    &callback,
                    &prepared,
                )
                .await
                .map_err(debug)?;
            let first:String=sqlx::query_scalar("SELECT item_instance_id::text FROM game_spell_item_lines WHERE transaction_id=encode($1,'hex')::uuid AND operation_kind=1")
                .bind(created.receipt.transaction_id.as_slice()).fetch_one(&h.pool).await?;
            drop(permit);

            // The existing Control owner advances the real source Channel after the
            // fire exists (the Global producer binds the latest activation); the
            // accepted Instance keeps its original entry pins.
            let RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } = current.runtime_scope
            else {
                return Err("actual source Channel".into());
            };
            let original = scene.pin.activation_sequence();
            let next = original.checked_add(1).ok_or("next source activation")?;
            assert!(
                h.root
                    .record_content_activation(
                        &crate::durability::content_activation::ContentActivationRequest {
                            world_id,
                            channel_id,
                            activation_sequence: next,
                            previous_sequence: Some(original),
                            server_artifact_digest: scene.pin.server_artifact_digest(),
                            client_artifact_digest: scene.pin.client_artifact_digest(),
                            frame_binding_digest: scene.pin.frame_binding_digest(),
                        },
                    )
                    .await?
            );
            assert_eq!(
                h.root
                    .read_current_content_activation(world_id, channel_id)
                    .await?
                    .ok_or("advanced source activation")?
                    .activation_sequence,
                next
            );
            assert_eq!(
                view.runtime
                    .lock()
                    .await
                    .content_pin()
                    .activation_sequence(),
                original,
                "the accepted Instance retains its own original entry pins"
            );


            // The actual paced STEP onto the field retains the Source STEP.
            assert!(view.quest_catalogue.is_none());
            view.clock_origin = view
                .clock_origin
                .checked_sub(std::time::Duration::from_millis(500))
                .ok_or("actual Spite approach pace")?;
            let (moved, _) = view
                .ordinary_paced_step(
                    actor,
                    admitted.game_session_id,
                    StepDirection::North,
                    None,
                    None,
                )
                .await;
            assert_eq!(moved.disposition, StepDisposition::Moved);
            let (input, step_fence) = view
                .retained_spite_step_for_test()
                .await
                .ok_or("actual retained Instance Source STEP")?;
            assert_eq!(
                input.item_instance_id(),
                sqlx::types::Uuid::parse_str(&first)?.into_bytes()
            );
            assert_eq!(step_fence.runtime_scope, destination);

            // Resolving through the sole Instance writer consumes it once.
            let lowered =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            view.quest_catalogue = Some(std::sync::Arc::new(lowered.catalogue().clone()));
            let _permit = view
                .spell_lane_permit()
                .await
                .ok_or("retained Instance Source STEP resolves")?;
            assert!(view.retained_spite_step_for_test().await.is_none());
            let row:(i64,i64)=sqlx::query_as("SELECT (SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),(SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_kind='source_step')")
                .bind(step_fence.character_id.as_bytes().as_slice()).bind(rule::SPITE_FIRE).fetch_one(&h.pool).await?;
            assert_eq!(
                row,
                (1_800_000_000, 1),
                "the Instance arm admits its historical source activation"
            );
            let retired:(i16,i64,bool)=sqlx::query_as("SELECT lifecycle,quantity,EXISTS(SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=$1::uuid")
                .bind(&first).fetch_one(&h.pool).await?;
            assert_eq!(retired, (2, 0, false));
            Ok(())
        })
    })
}

async fn move_player(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    destination: MovementLocalPosition,
) -> TestResult {
    // The same blocker selection as the paced Movement owner for this view.
    let (none, bundle_doors, legacy);
    let closed: &dyn crate::map::floor::CellBlocker = if owner
        .rooms()
        .gate_source_world()
        .is_some_and(|room| room.source_world().is_some())
    {
        none = std::collections::BTreeSet::<crate::content::LogicalCell>::new();
        &none
    } else if let Some(map) = owner.bundle_map() {
        bundle_doors = crate::map::floor::BundleDoors(&map.facts);
        &bundle_doors
    } else {
        legacy = owner.door.lock().await.blocking_cells().clone();
        &legacy
    };
    let mut runtime = owner.runtime.lock().await;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let proof = crate::map::floor::prepare_quest_encounter_ejection(
        &runtime,
        owner.movement_cells(),
        actor,
        session,
        before,
        destination,
        closed,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
