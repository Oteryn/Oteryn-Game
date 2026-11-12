//! The real source publisher and native handoff carry the qualifying pad reset
//! from the source Player to its new Instance. No fixture catalogue bypasses
//! the current Hatred source-hook qualification.
use super::TestResult;
use super::fixture::{Harness, debug, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::durability::activity_handoff::ActivityHandoffOwner;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use oteryn_protocol_oteryn::boss_encounter as wire;

#[test]
fn srx3_hatred_real_source_pad_resets_torment_before_native_handoff_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    run_hatred_entry(false)
}

#[test]
fn srx3_hatred_instance_sorrow_survives_later_source_channel_activation_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    run_hatred_entry(true)
}

fn run_hatred_entry(advance_source_activation: bool) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("srx3-hatred-entry", move || {
        run(async move |admin| {
            super::reset();
            let source = PinnedActivitySourceRoom::GoshnarsHatred;
            let scene = SourceScene::new(
                source,
                MovementLocalPosition {
                    x: 33773,
                    y: 31601,
                    floor: 14,
                },
            )?;
            let candidate = crate::content::bosses::activity_template::captured_boss_levers()
                .map_err(debug)?
                .into_iter()
                .find(|row| row.creature_key.as_deref() == Some("oteryn:creature.goshnar_s_hatred"))
                .ok_or("actual Hatred source candidate")?;
            let template = scene
                .controller
                .active()
                .ok_or("active Hatred generation")?
                .qualify_soul_war_activity_template(&scene.room, "content-1", source)
                .map_err(|error| format!("actual Hatred source template: {error:?}"))?;
            assert_eq!(
                Some(template.creature_key()),
                candidate.creature_key.as_deref()
            );
            assert_eq!(template.minimum_level(), 250);
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(admin, "srx3_hatred_entry", false, false)
                .await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i33793", 1)]).await?;
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
            .ok_or("native Hatred capabilities")?;
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
            let original_actor = admitted.runtime_actor.ok_or("actual source Player")?;
            {
                let runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let next = states
                    .get(&runtime, original_actor, admitted.game_session_id)
                    .ok_or("current source spell state")?
                    .stage_soul_war_torment_counter(24)
                    .ok_or("current source Torment predecessor")?;
                assert!(states.commit(&runtime, original_actor, admitted.game_session_id, next));
            }
            ActivityHandoffOwner::new(h.root.clone())
                .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
                .await?;
            owner
                .activity_runtimes
                .register_template(&template)
                .map_err(|_| "source template")?;
            let destination = super::activity_runtime::allocate_ready_instance(&h, 232).await?;
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
            let view = owner.runtime_view(&registered);
            let runtime = view.runtime.lock().await;
            let actor = admitted.runtime_actor.ok_or("actual transferred Player")?;
            let states = view.states.spell_states.lock().await;
            let player = states
                .get(&runtime, actor, admitted.game_session_id)
                .ok_or("actual transferred Player state")?;
            assert_eq!(player.soul_war_torment_count(), 0);
            assert_eq!(player.soul_war_torment_icon_count(), None);
            assert_eq!(
                states.soul_war_callbacks.hatred_entry_for_test(),
                Some((180, true))
            );
            assert_eq!(
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    x: 33743,
                    y: 31604,
                    floor: 14
                }
            );
            drop(states);
            drop(runtime);
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .player_control_facts(original_actor, current.game_session_id)
                    .is_err(),
                "the reset belongs to the transferred Player, not a fabricated destination state"
            );
            drop(view);
            if advance_source_activation {
                let crate::foundation::RuntimeScopeRefV1::Channel {
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
                        .await?,
                    "the existing Control owner advances the real source Channel issuance"
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
                    owner
                        .runtime_view(&registered)
                        .runtime
                        .lock()
                        .await
                        .content_pin()
                        .activation_sequence(),
                    original,
                    "the accepted Instance retains its own original entry pins"
                );
            }
            // Reach the actual Burning child through ordinary source-map STEP;
            // no runtime teleport or administrative boss catalogue seeds the USE.
            for _ in 0..4 {
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_millis(500))
                    .ok_or("actual Sorrow approach pace")?;
                let view = owner.runtime_view(&registered);
                let (step, _) = view
                    .ordinary_paced_step(
                        actor,
                        admitted.game_session_id,
                        oteryn_protocol_oteryn::world_spatial::StepDirection::North,
                        None,
                        None,
                    )
                    .await;
                assert_eq!(
                    step.disposition,
                    oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
                );
            }
            let view = owner.runtime_view(&registered);
            let (burning, boss, before) = {
                let runtime = view.runtime.lock().await;
                let visible = runtime.visible_entities();
                let find = |key: &[u8]| {
                    visible
                        .creatures
                        .iter()
                        .find(|entry| runtime.matches_live_creature_identity(entry.actor, key))
                        .map(|entry| entry.actor)
                };
                let burning = find(b"oteryn:creature.ashes_of_burning_hatred")
                    .ok_or("real initial Burning actor")?;
                let boss = find(b"oteryn:creature.goshnar_s_hatred").ok_or("real Hatred boss")?;
                let state = view
                    .states
                    .spell_states
                    .lock()
                    .await
                    .soul_war_callbacks
                    .hatred_entry_for_test()
                    .ok_or("real current Burning clock")?;
                (burning, boss, state.0)
            };
            let source_item = view
                .active_generation
                .and_then(|generation| generation.native_gameplay())
                .and_then(|native| native.item_index())
                .and_then(|index| {
                    index.definition_ref("Item", "oteryn:item.tibia.i33793", "definition-r1")
                })
                .ok_or("qualified original Sorrow Item")?;
            let intent = oteryn_protocol_oteryn::world_object::UseIntent {
                target: oteryn_protocol_oteryn::world_object::UseTarget::ItemByDefinition(
                    source_item,
                ),
                use_with: Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                    identity: burning.placement_identity(),
                    generation: burning.actor_local_generation(),
                }),
                use_with_map_item: None,
                use_with_position: None,
            };
            let wrong = oteryn_protocol_oteryn::world_object::UseIntent {
                use_with: Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                    identity: boss.placement_identity(),
                    generation: boss.actor_local_generation(),
                }),
                ..intent.clone()
            };
            let command = |command_id| crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: admitted.item_fence,
            };
            let rejected = view
                .dispatch_soul_war_source_use(actor, command(1), &wrong, None, None)
                .await
                .map_err(|_| "actual wrong Sorrow target")?
                .ok_or("source Sorrow selection")?;
            assert_eq!(
                rejected.disposition,
                oteryn_protocol_oteryn::world_object::UseDisposition::NotPossible
            );
            let quest_before: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            let accepted = view
                .dispatch_soul_war_source_use(actor, command(2), &intent, None, None)
                .await
                .map_err(|_| "actual Sorrow Item owner")?
                .ok_or("actual accepted Sorrow")?;
            assert_eq!(
                accepted.disposition,
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            );
            let replay = view
                .dispatch_soul_war_source_use(actor, command(2), &intent, None, None)
                .await
                .map_err(|_| "original Sorrow replay")?
                .ok_or("retained Sorrow replay")?;
            assert_eq!(
                replay.disposition,
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            );
            let runtime = view.runtime.lock().await;
            assert!(!runtime.actor_spell_reserved(actor));
            assert!(!runtime.actor_spell_reserved(burning));
            assert_eq!(
                view.states
                    .spell_states
                    .lock()
                    .await
                    .soul_war_callbacks
                    .hatred_entry_for_test(),
                Some((before + 10, true))
            );
            drop(runtime);
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
                    .fetch_one(&h.pool)
                    .await?,
                quest_before,
                "Sorrow has no durable Quest transition"
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_soul_war_source_use_proofs WHERE source_key='oteryn:item.tibia.i33793' AND accepted AND transition_key IS NULL AND quest_request_binding IS NULL")
                .fetch_one(&h.pool).await?,1);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_use_records WHERE cause_kind=5 AND source_definition_production_key='oteryn:item.tibia.i33793' AND source_quantity_before=1 AND source_quantity_after=0 AND channel_id IS NULL AND activity_instance_id IS NOT NULL")
                .fetch_one(&h.pool).await?,1);
            drop(view);
            drop(registered);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
