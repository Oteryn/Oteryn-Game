//! A refused Hatred lever still prepares the source room (boss plus the
//! lever-created Ashes, boss_lever.lua:228-229). A later valid lever must reuse
//! that unentered room instead of ending NoCapacity.
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
use oteryn_gameplay_encounters::bosses::goshnars_hatred::BurningForm;
use oteryn_protocol_oteryn::boss_encounter as wire;

const HATRED: &str = "oteryn:creature.goshnar_s_hatred";

#[test]
fn hatred_room_reused_after_refused_lever_accepts_its_helper_roster() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("srx3-hatred-room-reuse", move || {
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
            let template = scene
                .controller
                .active()
                .ok_or("active Hatred generation")?
                .qualify_soul_war_activity_template(&scene.room, "content-1", source)
                .map_err(|error| format!("actual Hatred source template: {error:?}"))?;
            let deps = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_hatred_room_reuse", false, false)
                    .await?;
            let evidence = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
            super::house_transfer_fixtures::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, source)?;
            super::boss_activity_fixtures::issue_content_and_reset(&h, &scene.pin).await?;
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
            let destination = super::activity_runtime_fixtures::allocate_ready_instance(&h, 233).await?;

            // Without the Soul War Start the endgame gate refuses this lever,
            // after the source room was already prepared in the only Instance.
            let (_, refused, barrier) = super::boss_activity_fixtures::exchange_capture(
                &owner,
                &mut admitted,
                template.boss_key(),
                false,
                &mut Vec::new(),
            )
            .await?;
            assert!(
                matches!(refused, wire::BossActivityReply::Refused(_)),
                "{refused:?}"
            );
            assert!(barrier.is_none());
            let prepared = owner
                .activity_runtimes
                .activity_owners()
                .map_err(|_| "activity owners")?
                .into_iter()
                .filter(|registered| registered.scope == destination)
                .collect::<Vec<_>>();
            assert_eq!(prepared.len(), 1, "the refused lever prepared one room");
            let prepared_generation = prepared[0].generation;
            {
                let runtime = prepared[0].runtime.lock().await;
                let census = runtime.positioned_actor_census().map_err(debug)?;
                assert_eq!(census.len(), 2, "Hatred plus its lever-created Ashes");
                for key in [HATRED, BurningForm::Ashes.key()] {
                    assert_eq!(
                        census
                            .iter()
                            .filter(|(actor, _, session)| {
                                session.is_none()
                                    && runtime
                                        .companion_snapshot(*actor)
                                        .is_ok_and(|view| view.state.policy.definition_key == key)
                            })
                            .count(),
                        1,
                        "{key}"
                    );
                }
            }
            drop(prepared);

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

            let (_, reply, barrier) = super::boss_activity_fixtures::exchange_capture(
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
            assert_eq!(
                fence.scope_ownership_generation.get(),
                prepared_generation,
                "the valid lever reuses the room the refused lever prepared"
            );
            let registered = owner
                .activity_runtimes
                .get(destination, fence.scope_ownership_generation.get())
                .ok_or("actual destination owner")?;
            {
                let runtime = registered.runtime.lock().await;
                let census = runtime.positioned_actor_census().map_err(debug)?;
                assert_eq!(
                    census
                        .iter()
                        .filter(|(actor, _, _)| {
                            runtime.companion_snapshot(*actor).is_ok_and(|view| {
                                view.state.policy.definition_key == BurningForm::Ashes.key()
                            })
                        })
                        .count(),
                    1,
                    "reuse spawns no second Ashes"
                );
            }
            drop(registered);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}
