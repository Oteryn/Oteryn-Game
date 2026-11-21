//! First entry reconciles vocation looks even when no familiar needs restoration.
use super::*;
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
    scenario("familiar_login_grant_look", true)
}

#[test]
fn familiar_first_entry_removes_expired_premium_vocation_look_postgres() -> TestResult {
    scenario("familiar_login_remove_look", false)
}

fn scenario(name: &'static str, premium_current: bool) -> TestResult {
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
                        content_revision: progression.context.content,
                        policy_revision: progression.policy_revision,
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
            Ok(())
        }
        .await;
        h.cleanup().await?;
        result
    })
}
