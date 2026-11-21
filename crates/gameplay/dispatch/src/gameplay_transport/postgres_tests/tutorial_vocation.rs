//! The Newhaven vocation offer and selection through admitted command dispatch.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::{admit, native_asset_revisions, publish_native_activation};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use oteryn_protocol_oteryn::CommandStatus;
use crate::gameplay_transport::connection::{ControlLossResult, FreshAdmissionAuthority, ResumeAttempt};

#[test]
fn tutorial_vocation_offer_and_submit_real_dispatch() -> TestResult {
    vocation_scenario(Case::Ordinary)
}

#[test]
fn tutorial_vocation_can_choose_again_after_a_missing_kit_revision() -> TestResult {
    vocation_scenario(Case::MissingKitRevision)
}

#[test]
fn tutorial_vocation_waits_for_main_backpack() -> TestResult {
    vocation_scenario(Case::MissingBag)
}

#[test]
fn tutorial_vocation_resumes_retained_request() -> TestResult {
    vocation_scenario(Case::Retained)
}

#[test]
fn tutorial_vocation_logout_keeps_refill_when_live_installation_is_pending() -> TestResult {
    vocation_scenario(Case::Uninstalled)
}

#[test]
fn tutorial_vocation_rejects_a_kit_that_would_exceed_the_carried_tree() -> TestResult {
    vocation_scenario(Case::Capacity)
}

#[test]
fn tutorial_vocation_holds_position_capture_during_the_initial_vitals_write() -> TestResult {
    vocation_scenario(Case::InitialVitalsHold)
}

#[test]
fn tutorial_vocation_retains_movement_during_periodic_save_and_logout() -> TestResult {
    vocation_scenario(Case::RetainedMovement)
}

#[test]
fn busy_retained_vocation_position_is_counted_as_failed() -> TestResult {
    vocation_scenario(Case::RetainedBusy)
}

#[test]
fn retained_vocation_position_can_move_back_to_the_cached_cell() -> TestResult {
    vocation_scenario(Case::RetainedBacktrack)
}

#[test]
fn stationary_retained_vocation_position_advances_the_check_deadline() -> TestResult {
    vocation_scenario(Case::RetainedStationary)
}

#[test]
fn retained_capture_rechecks_vocation_after_acquiring_revision_slot() -> TestResult {
    vocation_scenario(Case::RetainedCleared)
}

#[test]
fn retained_capture_is_not_reauthorised_after_resume() -> TestResult {
    vocation_scenario(Case::RetainedResumed)
}

#[test]
fn uncommitted_retained_vocation_saves_live_pools_on_checkpoint_and_logout() -> TestResult {
    vocation_scenario(Case::UncommittedPools)
}

#[test]
fn failed_initial_vitals_projection_invalidates_the_cached_position() -> TestResult {
    vocation_scenario(Case::FailedInitialProjection)
}

#[test]
fn retained_vocation_with_a_gone_actor_settles_the_position_flush() -> TestResult {
    vocation_scenario(Case::RetainedGone)
}

#[test]
fn uncommitted_stationary_checkpoint_cannot_hide_a_later_refill() -> TestResult {
    vocation_scenario(Case::UncommittedStationaryCache)
}

#[test]
fn committed_dawnport_trial_pools_survive_a_live_installation_miss() -> TestResult {
    vocation_scenario(Case::TrialRefillBeforeInstall)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Ordinary,
    MissingBag,
    MissingKitRevision,
    Retained,
    Uninstalled,
    Capacity,
    InitialVitalsHold,
    RetainedMovement,
    RetainedBusy,
    RetainedBacktrack,
    RetainedStationary,
    RetainedCleared,
    RetainedResumed,
    UncommittedPools,
    FailedInitialProjection,
    RetainedGone,
    UncommittedStationaryCache,
    TrialRefillBeforeInstall,
}

fn vocation_scenario(case: Case) -> TestResult {
    let post_commit = matches!(case, Case::Uninstalled | Case::RetainedMovement | Case::RetainedBusy | Case::RetainedBacktrack | Case::RetainedStationary | Case::RetainedCleared | Case::RetainedResumed | Case::UncommittedPools | Case::FailedInitialProjection | Case::RetainedGone | Case::UncommittedStationaryCache | Case::TrialRefillBeforeInstall);
    let retained = case == Case::Retained || post_commit;
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("tutorial-vocation", move || {
        run(async |admin| {
            let h = Harness::create_newhaven(admin, "tutorial_vocation").await?;
            let recovery = if retained {
                Some(super::house_transfer_fixtures::RecoverySource::start(&h).await?)
            } else {
                None
            };
            let request = if retained {
                super::house_transfer_fixtures::prepare_owned_session_request(&h).await?
            } else {
                prepare_current_session(&h).await?
            };
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) =
                super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
            publish_native_activation(&h, &pin).await?;
            let active = controller.active().ok_or("active")?;
            native_asset_revisions(&h, active).await?;
            let room = pin.qualified_room().clone();
            // The native test scene is small; use the accepted Newhaven entry start,
            // as the existing Corruptor qualification does, for this owner fixture.
            let channel_pin = crate::foundation::ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                room.map_revision_digest(),
                (32534, 32513, 7),
            );
            let deps = Dependencies::with_content(
                channel_pin,
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            if let Some(recovery) = &recovery {
                owner.evidence = &recovery.evidence;
            }
            let mut admitted = deps.admitted;
            Box::pin(owner.admit_progression_session(&mut admitted)).await;
            admit(&owner, &deps, &request).await?;
            let current =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            admitted.first_entry =
                crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
            assert_eq!(
                super::cyclopedia_fixtures::support::exchange(&owner, &mut admitted, 1, 6243, &[])
                    .await?
                    .0,
                CommandStatus::Rejected,
                "504 must be selected"
            );
            const OFFERS: &[OfferedCapability] = &[
                OfferedCapability {
                    id: 31,
                    requires: &[],
                },
                OfferedCapability {
                    id: 504,
                    requires: &[31],
                },
            ];
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(OFFERS, &[31, 504]).ok_or("select vocation")?;
            owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[], current.current_runtime_scope(), current.current_scope_generation().get(), "policy-1", OFFERS,
            ).map_err(debug)?;
            let (status, bytes) =
                super::cyclopedia_fixtures::support::exchange(&owner, &mut admitted, 2, 6243, &[]).await?;
            assert_eq!(
                status,
                CommandStatus::Accepted,
                "tutorial vocation observation"
            );
            assert_eq!(bytes, [0, 0], "no offer before entering the portal");
            assert_eq!(
                super::cyclopedia_fixtures::support::exchange(&owner, &mut admitted, 3, 6243, &[4])
                    .await?
                    .0,
                CommandStatus::Rejected,
                "cannot choose before the portal"
            );
            let landing = crate::foundation::MovementLocalPosition {
                x: 32544,
                y: 32515,
                floor: 7,
            };
            {
                let mut runtime = deps.runtime.lock().await;
                let mut movement = runtime.borrow_movement_position();
                let mut at = movement.read(deps.actor).map_err(debug)?;
                for x in 32535..=32544 {
                    let next = crate::foundation::MovementLocalPosition { x, ..at.position() };
                    at = movement.commit_cardinal(at, next).map_err(debug)?;
                }
                for y in 32514..=32515 {
                    let next = crate::foundation::MovementLocalPosition { y, ..at.position() };
                    at = movement.commit_cardinal(at, next).map_err(debug)?;
                }
                assert_eq!(at.position(), landing);
            }
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "tutorial fence")?
                .ok_or("tutorial fence")?;
            owner
                .reconcile_tutorial_position(fence, landing)
                .await
                .map_err(|()| "portal tutorial transition")?;
            let (status, offer) =
                super::cyclopedia_fixtures::support::exchange(&owner, &mut admitted, 4, 6243, &[]).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(offer, [31, 0]);
            assert_eq!(
                super::cyclopedia_fixtures::support::exchange(&owner, &mut admitted, 5, 6243, &[6])
                    .await?
                    .0,
                CommandStatus::Rejected
            );
            if case == Case::MissingKitRevision {
                use crate::gameplay_transport::tutorial_vocation::MISSING_KIT_REVISION;
                use oteryn_gameplay_items::starter_grant::newhaven_starter_items;
                let missing = newhaven_starter_items("knight")
                    .ok_or("knight kit")?.iter()
                    .find(|item| !newhaven_starter_items("paladin").is_some_and(|kit| {
                        kit.iter().any(|other| other.tibia_id == item.tibia_id)
                    }))
                    .ok_or("knight-only item")?.tibia_id;
                assert!(MISSING_KIT_REVISION.scope(missing, owner.tutorial_vocation_wire(admitted, &[1])).await.is_none());
                assert!(owner.states.tutorial_sessions.vocation(admitted.game_session_id).map_err(debug)?.is_none());
            }
            if case == Case::MissingBag {
                move_fixture_bag(&h, 41, 98).await?;
                assert!(
                    owner.tutorial_vocation_wire(admitted, &[4]).await.is_none(),
                    "kit could not be placed"
                );
                assert!(
                    owner.tutorial_vocation_wire(admitted, &[]).await.is_none(),
                    "observation retries the kit"
                );
                move_fixture_bag(&h, 98, 41).await?;
                fill_fixture_backpack(&h, true).await?;
                assert!(owner.tutorial_vocation_wire(admitted, &[4]).await.is_none());
                let build = h.root.read_character_build_state(&authority, fence.character_id).await.map_err(debug)?;
                assert_eq!(build.vocation(), crate::durability::character_build::NO_VOCATION);
                fill_fixture_backpack(&h, false).await?;

            }
            if case == Case::Capacity {
                fill_nested_fixture(&h, true).await?;
                assert!(owner.tutorial_vocation_wire(admitted, &[4]).await.is_none());
                let build = h.root.read_character_build_state(&authority, fence.character_id).await.map_err(debug)?;
                assert_eq!(build.vocation(), crate::durability::character_build::NO_VOCATION);
                fill_nested_fixture(&h, false).await?;
            }
            if retained {
                use crate::durability::character_build::{BuildFormula, BuildOccurrence};
                use crate::gameplay_transport::vocation_choice::{
                    RetainedNpcVocation, vocation_choice_request,
                };
                let formula = active
                    .native_gameplay()
                    .ok_or("active native gameplay")?
                    .training_formula()
                    .ok_or("training formula")?;
                let before = h
                    .root
                    .read_character_build_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                let request = vocation_choice_request(
                    BuildOccurrence::from_bytes(id(88)).map_err(debug)?,
                    before,
                    "paladin",
                    formula,
                    None,
                )
                .map_err(debug)?;
                let old = owner
                    .npc_vocation_fence(admitted.game_session_id)
                    .await
                    .ok_or("old fence")?;
                if case == Case::RetainedBacktrack {
                    let session = admitted.game_session_id;
                    let runtime = owner.runtime.lock().await;
                    let mut states = owner.states.spell_states.lock().await;
                    states.get_mut(&runtime, deps.actor, session).ok_or("spell state")?.set_health_for_test(1);
                    drop(states);
                    drop(runtime);
                    assert!(owner.track_restored(
                        session, deps.actor,
                        &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
                        fence,
                    ));
                    assert_eq!(owner.write_due_last_positions(true).await, 0);
                    move_tutorial_actor(&deps, landing.x + 1).await?;
                }
                assert!(owner.states.tutorial_sessions.retain_vocation(
                    admitted.game_session_id,
                    RetainedNpcVocation {
                        fence: old,
                        request,
                        formula_digest: formula.digest()
                    }
                ));
                if case != Case::RetainedBacktrack {
                assert!(owner.track_restored(
                    admitted.game_session_id,
                    deps.actor,
                    &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
                    fence,
                ));
                }
                if post_commit {
                    use crate::gameplay_transport::vocation_choice::commit_retained_npc_vocation;
                    let session = admitted.game_session_id;
                    {
                        let runtime = owner.runtime.lock().await;
                        let mut states = owner.states.spell_states.lock().await;
                        states.get_mut(&runtime, deps.actor, session).ok_or("live spell state")?.set_health_for_test(1);
                    }
                    let retained = owner.states.tutorial_sessions.vocation(session)
                        .map_err(|()| "retained choice")?.ok_or("retained choice")?;
                    assert!(owner.save_first_vocation_vitals(deps.actor, session, old, &retained.request.before).await);
                    if case == Case::RetainedBacktrack {
                        move_tutorial_actor(&deps, landing.x).await?;
                        assert_eq!(owner.write_due_last_positions(true).await, 0);
                        let saved: i32 = sqlx::query_scalar("SELECT tile_x FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
                            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                        assert_eq!(saved, landing.x);
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return h.cleanup().await;
                    }
                    if case == Case::RetainedGone {
                        deps.runtime.lock().await.remove_test_actor(deps.actor).map_err(debug)?;
                        assert_eq!(owner.write_due_last_positions(true).await, 0);
                        assert!(owner.last_position_checked_for_test(session).is_none());
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return h.cleanup().await;
                    }
                    if case == Case::UncommittedStationaryCache {
                        assert_eq!(owner.write_due_last_positions(true).await, 0);
                        assert_eq!(saved_vitals(&h).await?.0, 1);
                        let mut slot = owner.revision_sequencer.acquire(old.character_id).await;
                        commit_retained_npc_vocation(
                            &mut slot, owner.root, owner.character, owner.holder, &retained, formula,
                        ).await.map_err(debug)?;
                        assert!(saved_vitals(&h).await?.0 > 1);
                        owner.states.tutorial_sessions.clear_vocation(session, retained.request.occurrence);
                        drop(slot);
                        assert_eq!(owner.write_due_last_positions(true).await, 0);
                        assert_eq!(saved_vitals(&h).await?.0, 1, "the refill must invalidate the prior pool sample");
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return h.cleanup().await;
                    }
                    if case == Case::TrialRefillBeforeInstall {
                        use crate::durability::character_build::BuildCause;
                        owner.states.tutorial_sessions.clear_vocation(session, retained.request.occurrence);
                        // Trial actions above level 8 refill. Keep the current actor
                        // wounded to stand in for a failed post-commit cast-facts read.
                        let mut setup = h.pool.begin().await?;
                        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *setup).await?;
                        sqlx::query("UPDATE game_character_progression_state SET level=9,total_experience=6400 WHERE character_id=encode($1,'hex')::uuid")
                            .bind(id(41).as_slice()).execute(&mut *setup).await?;
                        setup.commit().await?;
                        let mut trial = retained.clone();
                        trial.request.cause = BuildCause::VocationTrial;
                        let mut slot = owner.revision_sequencer.acquire(old.character_id).await;
                        assert!(owner.commit_dawnport_trial(deps.actor, &mut slot, &trial, formula, 9).await);
                        drop(slot);
                        let refill = saved_vitals(&h).await?;
                        assert!(refill.0 > 1);
                        move_tutorial_actor(&deps, landing.x + 1).await?;
                        assert_eq!(owner.write_due_last_positions(true).await, 0);
                        assert_eq!(saved_vitals(&h).await?.0, refill.0);
                        let _ = owner.logout_session(admitted).await;
                        assert_eq!(saved_vitals(&h).await?.0, refill.0);
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return h.cleanup().await;
                    }
                    if case == Case::UncommittedPools {
                        {
                            let runtime = owner.runtime.lock().await;
                            let mut states = owner.states.spell_states.lock().await;
                            states.get_mut(&runtime, deps.actor, session).ok_or("live spell state")?.set_health_for_test(2);
                        }
                        assert_eq!(owner.write_due_last_positions(true).await, 0);
                        assert_eq!(saved_vitals(&h).await?.0, 2);
                        {
                            let runtime = owner.runtime.lock().await;
                            let mut states = owner.states.spell_states.lock().await;
                            states.get_mut(&runtime, deps.actor, session).ok_or("live spell state")?.set_health_for_test(3);
                        }
                        let _ = owner.logout_session(admitted).await;
                        assert_eq!(saved_vitals(&h).await?.0, 3);
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return h.cleanup().await;
                    }
                    if case == Case::FailedInitialProjection {
                        // Cache the local pools at A, then leave the durable row at B
                        // as a writer that lost its acknowledgement would.
                        owner.states.tutorial_sessions.clear_vocation(session, retained.request.occurrence);
                        assert_eq!(owner.write_due_last_positions(true).await, 0);
                        sqlx::query("UPDATE game_character_last_positions SET tile_x=tile_x+1, write_sequence=write_sequence+1 WHERE character_id=encode($1,'hex')::uuid")
                            .bind(id(41).as_slice()).execute(&h.pool).await?;
                        let holder = h.root.try_issue_semantic_pass()?;
                        assert!(!owner.save_first_vocation_vitals(deps.actor, session, old, &retained.request.before).await);
                        holder.run(|_, _| Box::pin(async { Ok(()) })).await?;
                        assert_eq!(owner.write_due_last_positions(true).await, 0);
                        let saved: i32 = sqlx::query_scalar("SELECT tile_x FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
                            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                        assert_eq!(saved, landing.x);
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return h.cleanup().await;
                    }
                    let mut slot = owner.revision_sequencer.acquire(old.character_id).await;
                    if matches!(case, Case::RetainedMovement | Case::Uninstalled) {
                        use crate::durability::character_revision_sequencer::NewhavenBuildContext;
                        use crate::durability::starter_grant::NewhavenKit;
                        use crate::gameplay_transport::{
                            item_ref_admission::NativeItemWeights, tutorial_vocation::newhaven_kit_definitions,
                        };
                        let native = active.native_gameplay().ok_or("native gameplay")?;
                        let definitions = newhaven_kit_definitions(native, retained.request.after.vocation())
                            .ok_or("kit definitions")?;
                        slot.commit_newhaven_build(
                            owner.root, owner.character, owner.holder, retained.fence, retained.request.clone(),
                            NewhavenBuildContext {
                                formula,
                                kit: NewhavenKit { content: &NativeItemWeights(native), definitions: &definitions },
                            },
                        ).await.map_err(debug)?;
                        assert_eq!(kit_grant_count(&h).await?, i64::try_from(definitions.len())?);
                    } else {
                        commit_retained_npc_vocation(
                            &mut slot, owner.root, owner.character, owner.holder, &retained, formula,
                        ).await.map_err(debug)?;
                    }
                    drop(slot);
                    // Stop at the same state a failed post-commit read leaves behind:
                    // the refill has committed while the live actor still has 1 HP.
                    let refill = saved_vitals(&h).await?;
                    assert!(refill.0 > 1, "first vocation refilled the wounded actor");
                    if matches!(case, Case::RetainedCleared | Case::RetainedResumed) {
                        use crate::gameplay_transport::last_position::{RETAINED_CAPTURE_PAUSE, RetainedCaptureBoundary};
                        move_tutorial_actor(&deps, landing.x + 1).await?;
                        let clear = case == Case::RetainedCleared;
                        let boundary = if clear { RetainedCaptureBoundary::BeforeSlot } else { RetainedCaptureBoundary::AfterCapture };
                        let entered = std::sync::Arc::new(tokio::sync::Notify::new());
                        let proceed = std::sync::Arc::new(tokio::sync::Notify::new());
                        let write = RETAINED_CAPTURE_PAUSE.scope((boundary, entered.clone(), proceed.clone()), owner.write_due_last_positions(true));
                        let change = async {
                            entered.notified().await;
                            let result = async {
                                if clear {
                                    let slot = owner.revision_sequencer.acquire(old.character_id).await;
                                    owner.states.tutorial_sessions.clear_vocation(session, retained.request.occurrence);
                                    drop(slot);
                                } else {
                                    // Publish the one durable fact that changes at resume. The
                                    // full resume path also waits for the revision lane, held by
                                    // the old periodic writer until after this boundary.
                                    sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid")
                                        .bind(session.as_bytes().as_slice()).execute(&h.pool).await?;
                                    let current = owner.current_quest_fence(session).await.map_err(|()| "successor fence")?.ok_or("successor fence")?;
                                    assert_eq!(current.connection_generation.get(), old.connection_generation.get() + 1);
                                }
                                Ok::<_, Box<dyn std::error::Error>>(())
                            }.await;
                            proceed.notify_one();
                            result
                        };
                        let (failures, changed) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                            tokio::join!(write, change)
                        }).await?;
                        changed?;
                        let (health, _, sequence) = saved_vitals(&h).await?;
                        if clear {
                            assert_eq!(failures, 0);
                            assert_eq!(health, 1);
                        } else {
                            assert_eq!(failures, 1);
                            assert_eq!(sequence, refill.2, "old capture cannot write under the resumed fence");
                        }
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return h.cleanup().await;
                    }
                    if matches!(case, Case::RetainedBusy | Case::RetainedStationary) {
                        if case == Case::RetainedBusy {
                            move_tutorial_actor(&deps, landing.x + 1).await?;
                            let slot = owner.revision_sequencer.acquire(old.character_id).await;
                            assert_eq!(owner.write_due_last_positions(true).await, 1);
                            drop(slot);
                            assert_eq!(owner.write_due_last_positions(true).await, 0);
                        } else {
                            let checked = owner.last_position_checked_for_test(session).ok_or("tracked actor")?;
                            assert_eq!(owner.write_due_last_positions(true).await, 0);
                            assert!(owner.last_position_checked_for_test(session).ok_or("tracked actor")? > checked);
                            assert_eq!(saved_vitals(&h).await?, refill);
                        }
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return h.cleanup().await;
                    }
                    if case == Case::RetainedMovement {
                        for (distance, logout) in [(1, false), (2, true)] {
                            let x = landing.x + distance;
                            {
                                let mut runtime = deps.runtime.lock().await;
                                let mut movement = runtime.borrow_movement_position();
                                let at = movement.read(deps.actor).map_err(debug)?;
                                movement.commit_cardinal(at, crate::foundation::MovementLocalPosition { x, ..at.position() }).map_err(debug)?;
                            }
                            if logout {
                                let _ = owner.logout_session(admitted).await;
                            } else {
                                assert_eq!(owner.write_due_last_positions(true).await, 0);
                            }
                            let saved: i32 = sqlx::query_scalar("SELECT tile_x FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
                                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                            assert_eq!(saved, x);
                            let (health, mana, _) = saved_vitals(&h).await?;
                            assert_eq!((health, mana), (refill.0, refill.1));
                        }
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return h.cleanup().await;
                    }
                    assert!(owner.last_position_capture_is_held(session).await);
                    assert_eq!(owner.write_due_last_positions(true).await, 0);
                    assert_eq!(saved_vitals(&h).await?, refill);
                    let _ = owner.logout_session(admitted).await;
                    assert_eq!(saved_vitals(&h).await?, refill, "logout must preserve the committed refill");
                    drop(owner);
                    drop(authority);
                    drop(seal);
                    return h.cleanup().await;
                }
                assert!(
                    owner.last_position_capture_is_held(admitted.game_session_id).await,
                    "the retained choice must fence stale vitals even if post-commit installation fails"
                );
                owner.states.command_ingress.admit(admitted.game_session_id);
                assert_eq!(
                    owner.lose_control(admitted, std::time::Duration::ZERO, None).await,
                    ControlLossResult::Recorded,
                );
                let token = super::house_transfer_fixtures::RecoverySource::token(88)?;
                let prior = admitted;
                admitted = owner.resume_session(ResumeAttempt {
                    game_session_id: prior.game_session_id,
                    recovery_material: token.as_bytes(),
                    transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[88; 16]).map_err(debug)?,
                    last_applied_server_sequence: prior.continuity.server_sequence,
                    supported_capabilities: prior.continuity.selected_capabilities.as_slice(),
                }).await.map_err(debug)?;
                assert_eq!(admitted.game_session_id, prior.game_session_id);
                assert_eq!(admitted.continuity.connection_generation, prior.continuity.connection_generation + 1);
            }
            if case == Case::InitialVitalsHold {
                let session = admitted.game_session_id;
                assert!(owner.track_restored(session, deps.actor,
                    &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin, fence));
                let mut blocker = h.pool.begin().await?;
                sqlx::query("LOCK TABLE game_character_last_positions IN ACCESS EXCLUSIVE MODE")
                    .execute(&mut *blocker).await?;
                let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *blocker).await?;
                {
                let choice = owner.tutorial_vocation_wire(admitted, &[4]);
                tokio::pin!(choice);
                let waiting = async {
                    loop {
                        let waiting: bool = sqlx::query_scalar(
                            "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE relation='game_character_last_positions'::regclass AND database=(SELECT oid FROM pg_database WHERE datname=current_database()) AND NOT granted AND $1=ANY(pg_blocking_pids(pid)))",
                        ).bind(blocker_pid).fetch_one(&h.pool).await?;
                        if waiting {
                            return Ok::<_, sqlx::Error>(());
                        }
                        tokio::task::yield_now().await;
                    }
                };
                tokio::select! {
                    result = tokio::time::timeout(std::time::Duration::from_secs(5), waiting) => result??,
                    result = &mut choice => return Err(format!("choice finished before the vitals write: {result:?}").into()),
                }
                assert!(owner.last_position_capture_is_held(session).await);
                let failures = tokio::time::timeout(
                    std::time::Duration::from_millis(500), owner.write_due_last_positions(true),
                ).await?;
                assert_eq!(failures, 1, "the busy revision slot must leave the position due");
                blocker.rollback().await?;
                assert!(choice.await.is_some());
                }
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            let (status, selected) = super::cyclopedia_fixtures::support::exchange(
                &owner,
                &mut admitted,
                6,
                6243,
                if retained { &[] } else { &[4] },
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted, "submit Paladin");
            assert_eq!(selected, [0, 4]);
            let build = h
                .root
                .read_character_build_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(build.vocation(), "paladin");
            assert_eq!(
                super::cyclopedia_fixtures::support::exchange(&owner, &mut admitted, 7, 6243, &[])
                    .await?
                    .1,
                selected
            );
            assert_eq!(
                super::cyclopedia_fixtures::support::exchange(&owner, &mut admitted, 8, 6243, &[1])
                    .await?
                    .0,
                CommandStatus::Rejected,
                "a selected vocation cannot be replaced"
            );
            let mut stale = admitted;
            stale.continuity.connection_generation += 1;
            assert!(
                owner.tutorial_vocation_wire(stale, &[]).await.is_none(),
                "a non-current generation cannot observe an offer"
            );
            // The connection owner rejects this generation before it can return a
            // command result. Keep the handler refusal and the transport closure covered.
            let refusal = super::cyclopedia_fixtures::support::exchange(&owner, &mut stale, 9, 6243, &[])
                .await
                .err()
                .ok_or("non-current connection returned a command result")?;
            let refusal = refusal.to_string();
            assert!(
                refusal.starts_with("cyclopedia command 9/6243 ended AdmittedThenDisconnected(")
            );
            assert!(refusal.ends_with("early eof"));
            let occurrences: Vec<Vec<u8>> = sqlx::query_scalar(
                "SELECT uuid_send(build_occurrence_id) FROM game_character_build_receipts WHERE cause='vocation_choice'",
            )
            .fetch_all(&h.pool)
            .await?;
            assert_eq!(occurrences.len(), 1);
            if retained {
                assert_eq!(occurrences[0], id(88));
            }
            let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_starter_grants WHERE template_key LIKE 'oteryn:starter.newhaven.%'").fetch_one(&h.pool).await?;
            assert!(grants > 0, "the accepted profession kit is granted");
            if let Some(directory) = std::env::var_os("OTERYN_TUTORIAL_VOCATION_CAPTURE") {
                let directory = std::path::Path::new(&directory);
                std::fs::create_dir_all(directory)?;
                std::fs::write(directory.join("offer.bin"), offer)?;
                std::fs::write(directory.join("selected.bin"), selected)?;
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

async fn move_fixture_bag(h: &Harness, from: u8, to: u8) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let moved=sqlx::query("UPDATE game_item_container_slots SET character_id=encode($2,'hex')::uuid WHERE character_id=encode($1,'hex')::uuid").bind(id(from).as_slice()).bind(id(to).as_slice()).execute(&mut *tx).await?;
    assert_eq!(moved.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

async fn fill_fixture_backpack(h: &Harness, fill: bool) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    if fill {
        sqlx::query("WITH bag AS (SELECT character_id,item_instance_id,world_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid),
            items AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                SELECT game_character_uuid_v7(),world_id,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,game_character_uuid_v7() FROM bag CROSS JOIN generate_series(1,20)
                RETURNING item_instance_id,world_id,minted_transaction_id)
            INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id)
                SELECT bag.character_id,bag.item_instance_id,items.item_instance_id,items.world_id,100+row_number() OVER (ORDER BY items.item_instance_id),items.minted_transaction_id FROM items CROSS JOIN bag")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
    } else {
        sqlx::query("WITH removed AS (DELETE FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid AND placement_ordinal>100 RETURNING item_instance_id)
            DELETE FROM game_item_instances WHERE item_instance_id IN (SELECT item_instance_id FROM removed)")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}


async fn move_tutorial_actor(deps: &Dependencies, x: i32) -> TestResult {
    let mut runtime = deps.runtime.lock().await;
    let mut movement = runtime.borrow_movement_position();
    let at = movement.read(deps.actor).map_err(debug)?;
    movement.commit_cardinal(at, crate::foundation::MovementLocalPosition { x, ..at.position() }).map_err(debug)?;
    Ok(())
}

async fn saved_vitals(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT health::bigint,mana::bigint,write_sequence::bigint FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}


async fn fill_nested_fixture(h: &Harness, fill: bool) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    if fill {
        let (character, bag, world): (String, String, String) = sqlx::query_as(
            "SELECT character_id::text,item_instance_id::text,world_id::text FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&mut *tx).await?;
        let (parent, minted): (String, String) = sqlx::query_as(
            "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),$1::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id::text,minted_transaction_id::text",
        ).bind(&world).fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,1000,$5::uuid)")
            .bind(character).bind(bag).bind(&parent).bind(&world).bind(minted).execute(&mut *tx).await?;
        sqlx::query("WITH items AS (
            INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
            SELECT game_character_uuid_v7(),$1::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,game_character_uuid_v7()
            FROM generate_series(1,489) RETURNING item_instance_id,world_id,minted_transaction_id
        ) INSERT INTO game_item_nested_entries(parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id)
        SELECT $2::uuid,item_instance_id,world_id,row_number() OVER (ORDER BY item_instance_id),minted_transaction_id FROM items")
            .bind(world).bind(parent).execute(&mut *tx).await?;
        let size: i64 = sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)")
            .bind(id(41).as_slice()).fetch_one(&mut *tx).await?;
        assert!((490..=500).contains(&size));
    } else {
        sqlx::query("WITH parent AS (
            DELETE FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid AND placement_ordinal=1000 RETURNING item_instance_id
        ), children AS (
            DELETE FROM game_item_nested_entries WHERE parent_item_instance_id IN (SELECT item_instance_id FROM parent) RETURNING item_instance_id
        ) DELETE FROM game_item_instances WHERE item_instance_id IN (SELECT item_instance_id FROM parent UNION ALL SELECT item_instance_id FROM children)")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn kit_grant_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_starter_grants WHERE character_id=encode($1,'hex')::uuid AND template_key LIKE 'oteryn:starter.newhaven.%'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
