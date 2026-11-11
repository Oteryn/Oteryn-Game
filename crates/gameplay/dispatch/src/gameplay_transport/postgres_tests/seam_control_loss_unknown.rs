//! Retain one committed loss through unreadable acknowledgements and publication.
use super::*;
use crate::durability::fresh_admission::ExpiredLossReleaseV1;
use crate::foundation::{ControlLossMark, GameSessionState};
use crate::gameplay_transport::connection::{ControlLossResult, FreshAdmissionAuthority};
use crate::gameplay_transport::control_loss::{
    LOSS_PUBLICATION_BARRIER, LOSS_RECONCILIATION_PROBE, LossReconciliationProbe,
};
use std::sync::{Arc, atomic::Ordering};

#[test]
fn seam_control_loss_unknown_ack_keeps_original_through_six_busy_reads_postgres() -> TestResult {
    kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "loss_unknown_retained", false, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let store = FreshAdmissionStore::from_root(h.root.clone());
        let session = deps.admitted.game_session_id;
        let probe = Arc::new(LossReconciliationProbe::default());
        {
            let mut loss = std::pin::pin!(LOSS_RECONCILIATION_PROBE.scope(
                probe.clone(),
                owner.lose_control(deps.admitted, Duration::ZERO, None),
            ));
            tokio::time::timeout(Duration::from_secs(20), async {
                tokio::select! {
                    _ = probe.committed.notified() => {}
                    result = &mut loss => panic!("loss ended before its real commit: {result:?}"),
                }
            })
            .await?;
            let committed = store.current_session(session).await?;
            assert_eq!(committed.session_state(), GameSessionState::Reconnectable);
            let epoch = committed.current_control_loss_epoch().ok_or("loss epoch")?;
            let original = store
                .owning_loss(session, epoch)
                .await?
                .ok_or("owning loss")?;
            assert_eq!(epoch.get(), 1);
            assert_eq!(
                original.0.observation.original_grace_deadline - original.0.observation.loss_origin,
                crate::gameplay_transport::SAME_SESSION_GRACE_SECONDS,
            );
            assert_eq!(owning_loss_receipts(&h, session).await?, 1);
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .player_control_facts(deps.actor, session)
                    .map_err(debug)?
                    .control_loss
                    .is_none()
            );
            // These failures come from the real root holder, not a synthetic
            // store error. The old five-read loop returns Unknown before six.
            let held = h.root.try_issue_semantic_pass()?;
            probe.continue_after_commit.notify_one();
            tokio::time::timeout(Duration::from_secs(10), async {
                tokio::select! {
                    _ = async {
                        while probe.busy_reads.load(Ordering::SeqCst) < 6 {
                            probe.read_failed.notified().await;
                        }
                    } => {}
                    result = &mut loss => panic!("committed loss was abandoned: {result:?}"),
                }
            })
            .await?;
            assert!(
                owner.runtime.try_lock().is_ok(),
                "retry holds no runtime lock"
            );
            assert!(
                owner.states.spell_states.try_lock().is_ok(),
                "retry holds no spell-state lock",
            );
            assert!(
                !owner
                    .states
                    .lost
                    .lock()
                    .map_err(debug)?
                    .contains_key(&session)
            );
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(20), &mut loss).await?,
                ControlLossResult::Recorded,
            );
            assert!(probe.busy_reads.load(Ordering::SeqCst) >= 6);
            assert_eq!(
                probe.commit_acknowledgements.load(Ordering::SeqCst),
                1,
                "reconciliation must not replay the loss write",
            );
            assert_eq!(store.current_session(session).await?, committed);
            assert_eq!(
                store.owning_loss(session, epoch).await?,
                Some(original.clone())
            );
            assert_eq!(owning_loss_receipts(&h, session).await?, 1);
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .player_control_facts(deps.actor, session)
                    .map_err(debug)?
                    .control_loss,
                Some(ControlLossMark {
                    epoch: epoch.get(),
                    grace_deadline: original.0.observation.original_grace_deadline,
                }),
            );
            assert!(
                owner
                    .states
                    .lost
                    .lock()
                    .map_err(debug)?
                    .contains_key(&session)
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn seam_control_loss_terminal_before_publication_never_marks_actor_postgres() -> TestResult {
    publication_superseded(true)
}

#[test]
fn seam_control_loss_retired_owner_before_publication_never_marks_actor_postgres() -> TestResult {
    publication_superseded(false)
}

fn publication_superseded(terminal: bool) -> TestResult {
    kill_reward_e2e::scenario_thread::run_async(async move |admin| {
        let h = Harness::create_with_admission_seed(
            admin,
            if terminal {
                "loss_terminal_readback"
            } else {
                "loss_retired_readback"
            },
            false,
            false,
        )
        .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let store = FreshAdmissionStore::from_root(h.root.clone());
        let session = deps.admitted.game_session_id;
        let barrier = Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
        {
            let mut loss = std::pin::pin!(LOSS_PUBLICATION_BARRIER.scope(
                barrier.clone(),
                owner.lose_control(deps.admitted, Duration::ZERO, None),
            ));
            tokio::time::timeout(Duration::from_secs(20), async {
                tokio::select! {
                    _ = barrier.0.notified() => {}
                    result = &mut loss => panic!("loss ended before publication: {result:?}"),
                }
            })
            .await?;
            let current = store.current_session(session).await?;
            let epoch = current.current_control_loss_epoch().ok_or("loss epoch")?;
            assert_eq!(current.session_state(), GameSessionState::Reconnectable);
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .player_control_facts(deps.actor, session)
                    .map_err(debug)?
                    .control_loss
                    .is_none()
            );
            let expected = if terminal {
                // Use the existing fenced terminal writer, never forge session
                // or receipt rows to simulate terminality.
                assert!(matches!(
                    store
                        .release_capability_mismatch(
                            session,
                            &crate::gameplay_transport::canonical_uuid(&id(40)),
                            epoch,
                        )
                        .await?,
                    ExpiredLossReleaseV1::Released { .. }
                ));
                ControlLossResult::Terminal
            } else {
                owner.runtime.lock().await.retire_owner_cycle();
                ControlLossResult::NotApplicable
            };
            barrier.1.notify_one();
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(20), &mut loss).await?,
                expected,
            );
            assert!(
                !owner
                    .states
                    .lost
                    .lock()
                    .map_err(debug)?
                    .contains_key(&session)
            );
            assert_eq!(owning_loss_receipts(&h, session).await?, 1);
            if terminal {
                assert_eq!(
                    store.current_session(session).await?.session_state(),
                    GameSessionState::Terminal,
                );
                assert!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .player_control_facts(deps.actor, session)
                        .map_err(debug)?
                        .control_loss
                        .is_none()
                );
            } else {
                assert_eq!(store.current_session(session).await?, current);
                assert!(owner.runtime.lock().await.owner_fence().is_err());
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn owning_loss_receipts(h: &Harness, session: GameSessionId) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_durability_admission_lifecycle_receipts
         WHERE substring(operation_key from 1 for
             octet_length('owning-loss-v1'::bytea || $1::bytea))
             = 'owning-loss-v1'::bytea || $1::bytea",
    )
    .bind(session.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?)
}
