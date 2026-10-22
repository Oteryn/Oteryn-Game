//! Real composed quest admission and cadence against native PostgreSQL.
#![allow(clippy::expect_used)]

use std::sync::Arc;

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};

#[path = "quest_admission_runtime_role_tests.rs"]
mod runtime_role;

#[derive(Clone, Copy)]
enum Case {
    Bound,
    RuntimeRole,
    Concurrent,
    Unbound,
    FailedReceipt,
    ReplacedGeneration,
    Removed,
    Terminal,
    UnknownFence,
}

#[test]
fn quest_admission_pays_pending_xp_once() -> TestResult {
    scenario(Case::Bound)
}
#[test]
fn quest_admission_serializes_competing_transitions_with_a_typed_loser() -> TestResult {
    scenario(Case::Concurrent)
}
#[test]
fn quest_admission_unbound_defers_once_then_bound_pays() -> TestResult {
    scenario(Case::Unbound)
}
#[test]
fn quest_admission_receipt_failure_retries_on_cadence() -> TestResult {
    scenario(Case::FailedReceipt)
}
#[test]
fn quest_admission_replaced_generation_refuses_award() -> TestResult {
    scenario(Case::ReplacedGeneration)
}
#[test]
fn quest_admission_removed_session_is_never_recreated() -> TestResult {
    scenario(Case::Removed)
}
#[test]
fn quest_admission_expired_release_discards_copy_and_retry() -> TestResult {
    scenario(Case::Terminal)
}
#[test]
fn quest_admission_unknown_fence_retains_copy_and_retries() -> TestResult {
    scenario(Case::UnknownFence)
}

fn scenario(case: Case) -> TestResult {
    std::thread::Builder::new()
        .name("quest-admission".into())
        .stack_size(64 << 20)
        .spawn(move || execute(case).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "quest admission thread panicked")?
        .map_err(Into::into)
}

fn execute(case: Case) -> TestResult {
    run(async move |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "quest_admission", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let (generation, _) = super::death_ml_loss_composed::production_generation_with_pin(
            dependencies.admitted.world_id,
            "content-1",
        )?;
        let mut owner = dependencies.owner(&harness, &authority);
        owner.active_generation = generation.active();
        let session = dependencies.admitted.game_session_id;
        let mut admitted = dependencies.admitted;
        owner.admit_progression_session(&mut admitted).await;
        let binding = owner
            .progression_binding(session)
            .ok_or("production progression binding")?;
        assert_ne!(binding.policy.reward_revision, "content-1");

        let catalogue = super::quest_runtime_fixture::catalogue()?;
        owner.quest_catalogue = Some(catalogue);
        let refresh = owner.refresh_quest_session(session, false);
        assert!(
            std::mem::size_of_val(&refresh) < 32 * 1024,
            "XP helper must stay heap-pinned"
        );
        drop(refresh);
        let key = super::quest_runtime_fixture::TRANSITION;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("live fence")?;
        let request = |command_id| -> TestResult<QuestTransitionRequest> {
            Ok(QuestTransitionRequest {
                transition_key: key.into(),
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(command_id).map_err(debug)?,
                )),
            })
        };
        if matches!(case, Case::RuntimeRole) {
            harness
                .root
                .try_issue_semantic_pass()
                .map_err(debug)?
                .run(|holder, _| {
                    Box::pin(async move {
                        sqlx::query("SET ROLE oteryn_game_runtime")
                            .execute(&mut **holder)
                            .await?;
                        let role: String = sqlx::query_scalar("SELECT current_user")
                            .fetch_one(&mut **holder)
                            .await?;
                        assert_eq!(role, "oteryn_game_runtime");
                        let can_update: bool = sqlx::query_scalar(
                            "SELECT has_table_privilege(current_user, \
                             'game_character_quest_xp_obligations', 'UPDATE')",
                        )
                        .fetch_one(&mut **holder)
                        .await?;
                        assert!(!can_update);
                        Ok(())
                    })
                })
                .await
                .map_err(debug)?;
        }
        let pending = if matches!(case, Case::Concurrent) {
            let commit = |command_id| {
                let owner = &owner;
                let catalogue = owner.quest_catalogue.as_ref();
                async move {
                    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                    slot.commit_quest_transition(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        request(command_id)?,
                        Arc::clone(catalogue.ok_or("catalogue")?),
                    )
                    .await
                    .map_err(debug)
                }
            };
            let (one, two) = super::fixture::join_two(commit(1), commit(2)).await;
            match (one?, two?) {
                (
                    receipt @ QuestTransitionOutcome::Committed(_),
                    QuestTransitionOutcome::Refused(
                        oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch,
                    ),
                )
                | (
                    QuestTransitionOutcome::Refused(
                        oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch,
                    ),
                    receipt @ QuestTransitionOutcome::Committed(_),
                ) => receipt,
                other => return Err(format!("competing outcomes: {other:?}").into()),
            }
        } else {
            harness
                .root
                .commit_character_quest_transition(
                    &authority,
                    &harness.node,
                    fence,
                    request(1)?,
                    Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
                )
                .await
                .map_err(debug)?
        };
        assert!(matches!(pending, QuestTransitionOutcome::Committed(_)));
        assert_eq!(counts(&harness).await?, (0, 0, 1));

        if matches!(case, Case::Unbound | Case::Terminal | Case::UnknownFence) {
            owner
                .progression_sessions
                .lock()
                .map_err(debug)?
                .remove(&session);
            owner.admit_quest_session(&admitted).await;
            owner.admit_quest_session(&admitted).await;
            let entries = owner.quest_sessions.lock().map_err(debug)?;
            let entry = entries.get(&session).ok_or("quest session")?;
            assert!(entry.experience_deferred);
            assert!(entry.retry_at.is_none());
            assert_eq!(entry.copy.as_ref().ok_or("copy")?.xp_obligations().len(), 1);
        }

        match case {
            Case::Unbound => {
                owner.admit_progression_session(&mut admitted).await;
                owner.refresh_quest_session(session, false).await;
            }
            Case::Bound | Case::RuntimeRole | Case::Concurrent => {
                owner.admit_quest_session(&admitted).await;
            }
            Case::FailedReceipt => {
                sqlx::query(
                    "ALTER TABLE game_character_xp_receipts RENAME TO quest_unavailable_receipts",
                )
                .execute(&harness.pool)
                .await?;
                owner.admit_quest_session(&admitted).await;
                assert_eq!(counts_without_receipts(&harness).await?, (0, 1));
                assert!(
                    owner
                        .quest_sessions
                        .lock()
                        .map_err(debug)?
                        .get(&session)
                        .ok_or("entry")?
                        .retry_at
                        .is_some()
                );
                sqlx::query(
                    "ALTER TABLE quest_unavailable_receipts RENAME TO game_character_xp_receipts",
                )
                .execute(&harness.pool)
                .await?;
                owner
                    .quest_sessions
                    .lock()
                    .map_err(debug)?
                    .get_mut(&session)
                    .ok_or("entry")?
                    .retry_at = Some(std::time::Instant::now());
                owner.refresh_due_quest_sessions().await;
            }
            Case::Terminal => {
                dependencies.loss(&harness, -10).await?;
                let store = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                    harness.root.clone(),
                );
                assert!(matches!(
                    store
                        .release_expired_loss(
                            session,
                            &crate::gameplay_transport::canonical_uuid(&id(40))
                        )
                        .await
                        .map_err(debug)?,
                    crate::durability::fresh_admission::ExpiredLossReleaseV1::Released { .. }
                ));
                for _ in 0..2 {
                    owner.refresh_quest_session(session, false).await;
                }
                let entries = owner.quest_sessions.lock().map_err(debug)?;
                let entry = entries.get(&session).ok_or("entry")?;
                assert!(entry.copy.is_none() && entry.retry_at.is_none());
            }
            Case::UnknownFence => {
                sqlx::query("ALTER TABLE game_durability_reconnect_sessions RENAME TO quest_unavailable_sessions").execute(&harness.pool).await?;
                owner.refresh_quest_session(session, false).await;
                {
                    let entries = owner.quest_sessions.lock().map_err(debug)?;
                    let entry = entries.get(&session).ok_or("entry")?;
                    assert!(entry.copy.is_some() && entry.retry_at.is_some());
                }
                sqlx::query("ALTER TABLE quest_unavailable_sessions RENAME TO game_durability_reconnect_sessions").execute(&harness.pool).await?;
            }
            Case::ReplacedGeneration | Case::Removed => {
                owner
                    .progression_sessions
                    .lock()
                    .map_err(debug)?
                    .remove(&session);
                owner.admit_quest_session(&admitted).await;
                owner
                    .progression_sessions
                    .lock()
                    .map_err(debug)?
                    .insert(session, Arc::clone(&binding));
                let pause = crate::gameplay_transport::quest_admission::test_pause::install();
                let refresh = owner.refresh_quest_session(session, false);
                let mutation = async {
                    pause.reached.notified().await;
                    if matches!(case, Case::Removed) {
                        owner.forget_quest_session(session);
                        owner
                            .progression_sessions
                            .lock()
                            .map_err(debug)?
                            .remove(&session);
                    } else {
                        sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1").execute(&harness.pool).await?;
                    }
                    pause.resume.notify_one();
                    Ok::<_, Box<dyn std::error::Error>>(())
                };
                let (_, result) = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    super::fixture::join_two(refresh, mutation),
                )
                .await?;
                result?;
                if matches!(case, Case::Removed) {
                    assert!(
                        !owner
                            .quest_sessions
                            .lock()
                            .map_err(debug)?
                            .contains_key(&session)
                    );
                    assert!(owner.progression_binding(session).is_none());
                } else {
                    assert!(
                        owner
                            .quest_sessions
                            .lock()
                            .map_err(debug)?
                            .get(&session)
                            .ok_or("entry")?
                            .retry_at
                            .is_none()
                    );
                }
            }
        }
        let paid = matches!(
            case,
            Case::Bound
                | Case::RuntimeRole
                | Case::Concurrent
                | Case::Unbound
                | Case::FailedReceipt
        );
        assert_eq!(
            counts(&harness).await?,
            if paid { (50, 1, 0) } else { (0, 0, 1) }
        );
        if paid {
            owner.admit_quest_session(&admitted).await;
            owner.refresh_quest_session(session, false).await;
            assert_eq!(counts(&harness).await?, (50, 1, 0));
            assert!(
                owner
                    .quest_sessions
                    .lock()
                    .map_err(debug)?
                    .get(&session)
                    .ok_or("entry")?
                    .copy
                    .as_ref()
                    .ok_or("copy")?
                    .xp_obligations()
                    .is_empty()
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn counts(harness: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT total_experience, (SELECT count(*) FROM game_character_xp_receipts), (SELECT count(*) FROM game_character_quest_xp_obligations) FROM game_character_progression_state").fetch_one(&harness.pool).await?)
}

async fn counts_without_receipts(harness: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT total_experience, (SELECT count(*) FROM game_character_quest_xp_obligations) FROM game_character_progression_state").fetch_one(&harness.pool).await?)
}
