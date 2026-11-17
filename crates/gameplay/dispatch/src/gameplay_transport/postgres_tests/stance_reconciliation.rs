//! A committed stance survives an unrelated CharacterRevision without paying twice.
use super::death_ml_loss_composed::current_fence;
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session, reset, track};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, BuildOccurrence,
    DurableBuildState, convert_vocation,
};
use crate::durability::character_stance::{StanceChangeOccurrence, StanceChangeOutcome};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ExactActorRef, GameSessionId, GameSessionState};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::actor_spell::TrainingSave;
use crate::gameplay_transport::connection::LogoutReply;
use crate::gameplay_transport::spell_character_facts::{CastFactsLoad, load_character_cast_facts};
use crate::spell::SpellBook;
use crate::spell::cast::PlayerSpellState;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellCastIntent, SpellTarget};
use oteryn_protocol_oteryn::logout::LogoutResult;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

#[derive(Clone, Copy)]
enum Case {
    UnrelatedRevision,
    RetiredOwner,
    SupersededStance,
}

#[test]
fn stance_readback_after_stamina_revision_pays_once_and_releases_postgres() -> TestResult {
    scenario(Case::UnrelatedRevision)
}

#[test]
fn stance_readback_after_revision_advance_refuses_retired_owner_postgres() -> TestResult {
    scenario(Case::RetiredOwner)
}

#[test]
fn stance_readback_refuses_superseded_stance_and_same_key_aba_postgres() -> TestResult {
    scenario(Case::SupersededStance)
}

fn scenario(case: Case) -> TestResult {
    run_async(async move |admin| {
        reset();
        let tag = match case {
            Case::UnrelatedRevision => "stance_stamina_readback",
            Case::RetiredOwner => "stance_retired_readback",
            Case::SupersededStance => "stance_superseded_readback",
        };
        let h = Harness::create_with_admission_seed(admin, tag, true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        deps.spells = protector_book()?;
        let (generation, _) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let active = generation.active().ok_or("activated generation")?;
        let formula = active
            .native_gameplay()
            .and_then(|native| native.training_formula())
            .ok_or("training formula")?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;

        // Establish a real eligible build through the existing fenced writers.
        // Neither a receipt nor a session/current-revision row is fabricated.
        choose_elite_knight(&owner, session, formula).await?;
        let initial_fence = current_fence(&owner, session).await?;
        let CastFactsLoad::Ready { facts, .. } =
            load_character_cast_facts(owner.root, owner.character, initial_fence.character_id)
                .await
        else {
            return Err("owned cast facts unavailable".into());
        };
        let build = owner
            .root
            .read_character_build_state(owner.character, initial_fence.character_id)
            .await
            .map_err(debug)?;
        let stamina = owner
            .root
            .read_hunting_stamina(owner.character, initial_fence.character_id)
            .await
            .map_err(debug)?;
        assert!(stamina > 0);
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                    .is_some()
            );
            assert!(states.load_owned_build(&runtime, actor, session, &build));
            states.load_stamina_at(&runtime, actor, session, stamina, 0);
            // Retain one real stamina-owner debit, as after the first acknowledged hunt.
            states.hunting_experience(&runtime, actor, session, 1_000_000);
            assert_eq!(states.live_stamina(actor, session), Some(stamina - 1));
        }
        track(&owner, &deps)?;
        let before = state_snapshot(&owner, actor, session).await?;
        let intent = SpellCastIntent {
            spell: NonZeroU32::new(1).ok_or("stance index")?,
            target: SpellTarget::None,
            aim_at_target: false,
        };
        let pause = Arc::new((Notify::new(), Notify::new()));
        let (prepared, receipt) = {
            let mut cast = Box::pin(owner.cast_stance_with_readback_pause(
                actor,
                session,
                91,
                intent,
                Arc::clone(&pause),
            ));
            tokio::time::timeout(Duration::from_secs(20), async {
                tokio::select! {
                    () = pause.0.notified() => Ok(()),
                    result = &mut cast => Err(format!("cast ended before its receipt: {result:?}")),
                }
            })
            .await??;
            assert!(owner.runtime.try_lock().is_ok());
            assert!(owner.states.spell_states.try_lock().is_ok());
            let retained = state_snapshot(&owner, actor, session).await?;
            let prepared = retained.pending_stance().cloned().ok_or("pending stance")?;
            let receipt = owner
                .root
                .reconcile_character_stance(owner.character, prepared.request.occurrence)
                .await
                .map_err(debug)?
                .ok_or("committed stance receipt")?;
            assert!(prepared.matches_receipt(&receipt));
            assert_eq!(
                receipt.committed_character_revision().get(),
                initial_fence.expected_character_revision.get() + 1
            );
            assert_eq!(stance_receipts(&h, initial_fence.character_id).await?, 1);

            // A genuine busy holder makes the first post-commit read fail.
            // The cast must return with the original pending intent and no payment.
            let held = owner.root.try_issue_semantic_pass()?;
            pause.1.notify_one();
            let result = tokio::time::timeout(Duration::from_secs(20), &mut cast).await?;
            assert_eq!(result.disposition, SpellCastDisposition::Rejected);
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            assert_eq!(state_snapshot(&owner, actor, session).await?, retained);
            (prepared, receipt)
        };
        assert_eq!(
            state_snapshot(&owner, actor, session).await?.vitals(),
            before.vitals()
        );

        // This is the saver reached after stance reconciliation and before the
        // pending-spell guard in reconcile_spells_before_control_loss.
        assert_eq!(
            owner.save_hunting_stamina(actor, session).await,
            TrainingSave::Saved
        );
        let advanced = current_fence(&owner, session).await?;
        assert_eq!(
            advanced.expected_character_revision.get(),
            receipt.committed_character_revision().get() + 1
        );
        assert_eq!(
            owner
                .root
                .read_hunting_stamina(owner.character, advanced.character_id)
                .await
                .map_err(debug)?,
            stamina - 1,
        );
        let durable = owner
            .root
            .read_character_stance(owner.character, advanced.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(durable.key(), receipt.after());
        assert_eq!(
            durable.committed_character_revision(),
            Some(receipt.committed_character_revision())
        );
        assert!(
            state_snapshot(&owner, actor, session)
                .await?
                .pending_stance()
                .is_some()
        );

        match case {
            Case::UnrelatedRevision => {
                // Intervening HP is mutable owner state, not a stale paid predecessor.
                {
                    let runtime = owner.runtime.lock().await;
                    let mut states = owner.states.spell_states.lock().await;
                    let state = states.get_mut(&runtime, actor, session).ok_or("actor")?;
                    let (wounded, damage) = state.after_creature_damage(1).ok_or("damage")?;
                    assert_eq!(damage.applied, 1);
                    *state = wounded;
                }
                let result = owner
                    .reconcile_pending_stance_for_control_loss(actor, session)
                    .await
                    .ok_or("stance reconciliation")?;
                assert_eq!(result.disposition, SpellCastDisposition::Cast);
                let committed = state_snapshot(&owner, actor, session).await?;
                assert!(committed.pending_stance().is_none());
                assert_eq!(committed.durable_stance_key(), Some("protector"));
                assert_eq!(committed.vitals().mana, before.vitals().mana - 20);
                assert_eq!(committed.vitals().soul, before.vitals().soul);
                assert_eq!(committed.vitals().health, before.vitals().health - 1);
                assert!(
                    owner
                        .reconcile_pending_stance_for_control_loss(actor, session)
                        .await
                        .is_none()
                );
                assert_eq!(state_snapshot(&owner, actor, session).await?, committed);
                assert_eq!(stance_receipts(&h, advanced.character_id).await?, 1);
                assert_eq!(current_fence(&owner, session).await?, advanced);

                // The full logout owner needs the live admission item binding too.
                let mut admitted = deps.admitted;
                admitted.item_fence = Some(advanced.item_fence());
                assert_eq!(
                    owner.logout_session(admitted).await,
                    LogoutReply::Result(LogoutResult::Accepted)
                );
                assert_eq!(
                    FreshAdmissionStore::from_root(h.root.clone())
                        .current_session(session)
                        .await?
                        .session_state(),
                    GameSessionState::Terminal
                );
                assert!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .player_control_facts(actor, session)
                        .is_err()
                );
                assert_eq!(stance_receipts(&h, advanced.character_id).await?, 1);
            }
            Case::RetiredOwner => {
                let retained = state_snapshot(&owner, actor, session).await?;
                owner.runtime.lock().await.retire_owner_cycle();
                let result = owner
                    .reconcile_pending_stance_for_control_loss(actor, session)
                    .await;
                assert!(
                    result
                        .is_none_or(|result| result.disposition == SpellCastDisposition::Rejected)
                );
                assert_eq!(state_snapshot(&owner, actor, session).await?, retained);
                assert_eq!(stance_receipts(&h, advanced.character_id).await?, 1);
            }
            Case::SupersededStance => {
                let retained = state_snapshot(&owner, actor, session).await?;
                // Each later stance uses a new legal current-fence write. Returning
                // to the same key cannot make the original receipt current again.
                for (tag, before_key, after_key, count) in [
                    (203, Some("protector"), None, 2),
                    (204, None, Some("protector"), 3),
                ] {
                    let current = current_fence(&owner, session).await?;
                    let mut request = prepared.request.clone();
                    request.occurrence =
                        StanceChangeOccurrence::from_bytes(id(tag)).map_err(debug)?;
                    request.before = before_key.map(str::to_owned);
                    request.after = after_key.map(str::to_owned);
                    assert!(matches!(
                        owner
                            .root
                            .commit_character_stance(
                                owner.character,
                                owner.holder,
                                current,
                                request
                            )
                            .await
                            .map_err(debug)?,
                        StanceChangeOutcome::Committed(_),
                    ));
                    let result = owner
                        .reconcile_pending_stance_for_control_loss(actor, session)
                        .await
                        .ok_or("retained stance")?;
                    assert_eq!(result.disposition, SpellCastDisposition::Rejected);
                    assert_eq!(state_snapshot(&owner, actor, session).await?, retained);
                    assert_eq!(stance_receipts(&h, advanced.character_id).await?, count);
                }
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn protector_book() -> TestResult<SpellBook> {
    // Match the existing spell_stance_runtime_postgres_cases fixture: only its
    // Premium header is opened; the native stance policy and paid cost are real.
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    ))?;
    let profile = document["profiles"]
        .as_array()
        .ok_or("profiles")?
        .iter()
        .find(|profile| profile["name"] == "Protector")
        .ok_or("Protector")?;
    let mut spell = oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
        &serde_json::json!({"spell": profile["spell"]}),
        &profile["dependencies"],
    )
    .map_err(debug)?;
    spell.premium = false;
    SpellBook::new(vec![spell]).map_err(debug)
}

async fn choose_elite_knight(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    formula: &dyn BuildFormula,
) -> TestResult {
    for (tag, vocation, cause) in [
        (201, "knight", BuildCause::VocationChoice),
        (202, "elite_knight", BuildCause::Promotion),
    ] {
        let fence = current_fence(owner, session).await?;
        let _slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let before = owner
            .root
            .read_character_build_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        let after = if cause == BuildCause::VocationChoice {
            convert_vocation(formula, &before, vocation).map_err(debug)?
        } else {
            DurableBuildState::new(vocation, before.magic(), before.skills()).map_err(debug)?
        };
        let request = BuildChangeRequest {
            occurrence: BuildOccurrence::from_bytes(id(tag)).map_err(debug)?,
            cause,
            before,
            after,
            pruned_stance: None,
        };
        assert!(matches!(
            owner
                .root
                .commit_character_build(owner.character, owner.holder, fence, request, formula)
                .await
                .map_err(debug)?,
            BuildCommitOutcome::Committed(_),
        ));
    }
    Ok(())
}

async fn state_snapshot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<PlayerSpellState> {
    let _runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    // Inspect the exact retained entry after retirement as well: production get()
    // must refuse that owner, but the negative assertion must still see its state.
    states
        .actors
        .iter()
        .find(|(present, held, _)| *present == actor && *held == session)
        .map(|(_, _, state)| state.clone())
        .ok_or_else(|| "retained player state missing".into())
}

async fn stance_receipts(h: &Harness, character: crate::domain::CharacterId) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_stance_receipts WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?)
}
