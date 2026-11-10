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
    super::kill_reward_e2e::scenario_thread::run("srx3-hatred-entry", || {
        run(async |admin| {
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
                .qualify_activity_template(&scene.room, &scene.room, "content-1", &candidate)
                .map_err(|error| format!("actual Hatred source template: {error:?}"))?;
            assert_eq!(template.minimum_level(), 250);
            let deps = scene.dependencies()?;
            let h = Harness::create_with_admission_seed(admin, "srx3_hatred_entry", false, false)
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
            drop(registered);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
