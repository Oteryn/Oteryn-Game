//! First entry reconciles vocation looks even when no familiar needs restoration.
use super::*;
use crate::ability::creature_bite::CreatureBiteVitals;
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, convert_vocation,
};
use crate::durability::character_familiar::{
    DurableFamiliarState, FamiliarLifecycleV2, FamiliarLifecycleWriter, FamiliarStateOccurrence,
    FamiliarStateRequest,
};
use crate::durability::premium_fence::EntitlementState;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use crate::premium::{PremiumClass, PremiumConsumer};
use crate::spell::Execution;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use std::sync::Arc;

#[test]
fn familiar_first_entry_grants_and_selects_vocation_look_postgres() -> TestResult {
    scenario("familiar_login_grant_look", true, false)
}

#[test]
fn familiar_first_entry_removes_expired_premium_vocation_look_postgres() -> TestResult {
    scenario("familiar_login_remove_look", false, false)
}

#[test]
fn shutdown_checkpoint_retires_familiar_for_pending_player_death() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario("shutdown_familiar_death", true, true)
}

fn scenario(name: &'static str, premium_current: bool, shutdown_death: bool) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
        let result: TestResult = async {
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = fence(1)?.runtime_scope.world_id();
            let (generation, pin) =
                death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
            let active = generation.active().ok_or("active generation")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
            tutorial_vitals_fixture::native_asset_revisions(&h, active).await?;
            let room = pin.qualified_room().clone();
            let (channel_pin, _, _, _) = pin.into_channel_parts();
            let deps = Dependencies::with_content(channel_pin, room, native.spell_book().clone())?;
            let session = deps.admitted.game_session_id;
            let (consumer, clock, activation, mut evidence) = purchase_fixture::current(id(40));
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
            let consumer = if premium_current {
                consumer
            } else {
                evidence.state = EntitlementState::Expired;
                evidence.effective_until_us = evidence.authority_issued_at_us;
                let expired = Arc::new(PremiumConsumer::default());
                expired.prove_for_test(evidence);
                assert_eq!(
                    expired.class(id(40), clock.read_current_window()),
                    PremiumClass::Expired,
                );
                expired
            };
            let premium = SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            );
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            let mut admitted = deps.admitted;
            owner.admit_progression_session(&mut admitted).await;
            qengine_outfit_stories_fixtures::earn_level(&owner, &admitted, &authority, 200)
                .await?;

            let formula = native.training_formula().ok_or("training formula")?;
            let build_fence = death_ml_loss_composed_fixtures::current_fence(&owner, session).await?;
            let before = h
                .root
                .read_character_build_state(&authority, build_fence.character_id)
                .await
                .map_err(debug)?;
            let after = convert_vocation(formula, &before, "knight").map_err(debug)?;
            let build = h
                .root
                .commit_character_build(
                    &authority,
                    &h.node,
                    build_fence,
                    BuildChangeRequest {
                        occurrence: BuildOccurrence::from_bytes(id(182)).map_err(debug)?,
                        cause: if before.vocation() == "none" {
                            BuildCause::VocationChoice
                        } else {
                            BuildCause::VocationTrial
                        },
                        before,
                        after,
                        pruned_stance: None,
                    },
                    formula,
                )
                .await
                .map_err(debug)?;
            assert!(matches!(build, BuildCommitOutcome::Committed(_)));

            owner.premium_coordinator = Some(&premium);
            let index = owner
                .active_familiar_index("knight")
                .ok_or("knight familiar")?;
            let (spell, _) = owner.spells.source_indexed(index).ok_or("familiar spell")?;
            let Execution::NativeProfile(profile) = &spell.execution else {
                return Err("native familiar profile required".into());
            };
            let parameters = &profile.spell()["execution"]["native_behavior"]["parameters"];
            let look = parameters["default_look_type"]
                .as_u64()
                .ok_or("vocation look")?;
            let look = u32::try_from(look)?;

            assert_eq!(
                premium.premium_gate(id(40)).await.premium_current(),
                premium_current
            );
            let character = fence(1)?.character_id;
            let progression = h
                .root
                .read_character_progression(&authority, character)
                .await
                .map_err(debug)?
                .ok_or("level-200 progression")?;
            assert_eq!(progression.level, 200);
            let gameplay_fence =
                death_ml_loss_composed_fixtures::current_fence(&owner, session).await?;
            let before = DurableFamiliarState::default();
            let mut checkpoint = DurableFamiliarState {
                lifecycle: Some(FamiliarLifecycleV2 {
                    version: 2,
                    familiar_remaining_ms: 0,
                    open: false,
                    writer: FamiliarLifecycleWriter::from_fence(&gameplay_fence).map_err(debug)?,
                    clean_end_game_session_id: None,
                }),
                ..before.clone()
            };
            if !premium_current {
                checkpoint.selected_look = look;
                checkpoint.granted_looks.push(look);
            }
            h.root
                .commit_character_familiar_state(
                    &authority,
                    &h.node,
                    gameplay_fence,
                    FamiliarStateRequest {
                        occurrence: FamiliarStateOccurrence::from_bytes(id(183)).map_err(debug)?,
                        before,
                        after: checkpoint,
                        content_revision: progression.context.content.clone(),
                        policy_revision: progression.policy_revision.clone(),
                        policy_digest: native.source_digest(),
                    },
                )
                .await
                .map_err(debug)?;

            // Channel boot installs these in production (channel_boot.rs); Login needs them.
            native
                .install_companion_policies(&mut *owner.runtime.lock().await)
                .map_err(debug)?;
            tutorial_vitals_fixture::admit(&owner, &deps, &request).await?;
            let after = h
                .root
                .read_character_familiar_state(&authority, character)
                .await
                .map_err(debug)?;
            if premium_current {
                assert_eq!(after.state().granted_looks, [look]);
                assert_eq!(after.state().selected_look, look);
            } else {
                assert!(after.state().granted_looks.is_empty());
            }
            if shutdown_death {
                let definition = profile.familiar_creature_definition().ok_or("production familiar creature")?;
                let fence = death_ml_loss_composed_fixtures::current_fence(&owner, session).await?;
                let before = after.state().clone();
                let mut summoned = before.clone();
                summoned.lifecycle = Some(FamiliarLifecycleV2 {
                    version: 2,
                    familiar_remaining_ms: 60_000,
                    open: true,
                    writer: FamiliarLifecycleWriter::from_fence(&fence).map_err(debug)?,
                    clean_end_game_session_id: None,
                });
                summoned.lifecycle_epoch = before.lifecycle_epoch + 1;
                summoned.familiar_definition = Some(definition.key.clone());
                summoned.familiar_revision = Some(definition.revision);
                let epoch = summoned.lifecycle_epoch;
                h.root.commit_character_familiar_state(
                    &authority, &h.node, fence,
                    FamiliarStateRequest {
                        occurrence: FamiliarStateOccurrence::from_bytes(id(184)).map_err(debug)?,
                        before,
                        after: summoned,
                        content_revision: progression.context.content,
                        policy_revision: progression.policy_revision,
                        policy_digest: native.source_digest(),
                    },
                ).await.map_err(debug)?;
                owner.revision_sequencer =
                    crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new().into();
                {
                    let mut runtime = owner.runtime.lock().await;
                    let mut position = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
                    position.x += 1;
                    let spawn = runtime.prepare_companion_spawn(
                        deps.actor, session, &definition.key, position, Some(look),
                        None, epoch, 0, true,
                    ).map_err(debug)?;
                    let familiar = runtime.install_companion_spawn(spawn).map_err(debug)?;
                    assert!(runtime.companion_snapshot(familiar).map_err(debug)?.state.policy.is_familiar);
                    let mut states = owner.states.spell_states.lock().await;
                    let hit = states.apply_creature_damage(
                        &mut runtime, deps.actor, session, 100_000,
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(owner.owner_now().get()),
                    ).ok_or("lethal damage")?;
                    assert_eq!(hit.damage.health_after, 0);
                }
                let flush = crate::gameplay_transport::LastPositionFlush::default();
                flush.request();
                assert!(flush.serve(
                    &owner,
                    &crate::gameplay_transport::serving::QuiescedKillOwners::for_test(),
                ).await);
                let state = h.root.read_character_familiar_state(&authority, character).await.map_err(debug)?;
                let lifecycle = state.state().lifecycle.as_ref().ok_or("familiar lifecycle")?;
                assert!(!lifecycle.open, "the next login must not restore the familiar");
                assert_eq!(lifecycle.familiar_remaining_ms, 0);
                let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_pending_respawns")
                    .fetch_one(&h.pool).await?;
                assert_eq!(pending, 1);
                eprintln!("SHUTDOWN_FAMILIAR_DEATH_POSTGRES_CASE_COMPLETED");
            }
            Ok(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}
