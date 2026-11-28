//! NODEBOOT-1: admission entry steps and command gating reads wait for a busy durability
//! holder instead of refusing at once.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, track};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::AuthenticatedTransportRefV1;
use crate::gameplay_transport::AdmissionRetryBudget;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use std::time::Duration;

#[test]
fn a_committed_actor_enters_after_a_busy_holder_is_returned_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("first-entry-busy-holder", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "first_entry_busy_holder", false, false)
                    .await?;
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            let session = dependencies.admitted.game_session_id;
            // One background pass holds the holder past the whole blind retry span the first
            // entry used to have (RECONCILE_ATTEMPTS rounds of RECONCILE_BACKOFF).
            let busy = hold_holder(&h.root, Duration::from_millis(1_500))?;
            let budget = AdmissionRetryBudget::for_root(&session, &h.root);
            let (outcome, item_fence, _) = Box::pin(owner.enter_committed_actor(
                &budget,
                &request,
                session,
                AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?,
                dependencies.actor,
            ))
            .await;
            busy.await??;
            assert!(
                matches!(
                    outcome,
                    FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
                ),
                "{outcome:?}"
            );
            assert!(item_fence.is_some());
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn trade_open_retains_foreground_demand_until_its_name_reads_complete_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("trade-open-busy-holder", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "trade_open_busy_holder", true, false)
                    .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            dependencies.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            let held = h.root.try_issue_background_semantic_pass().map_err(debug)?;
            {
                let opening = crate::durability::DurabilityRoot::with_holder_owner(
                    owner.trade_open(dependencies.admitted),
                );
                let mut opening = std::pin::pin!(opening);
                let first = std::future::poll_fn(|cx| {
                    std::task::Poll::Ready(std::future::Future::poll(opening.as_mut(), cx))
                })
                .await;
                assert!(
                    first.is_pending(),
                    "trade open must wait for the busy holder"
                );
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                // Reproduce the scheduler retaking the holder before the waiting name
                // reader is polled again. Its FIFO ticket alone does not stop background passes.
                for _ in 0..16 {
                    assert!(matches!(
                        h.root.try_issue_background_semantic_pass(),
                        Err(crate::durability::DurabilityError::RootUnavailable)
                    ));
                }
                assert_eq!(opening.await, Some(()));
            }
            // A completed admission must release its preference for ordinary scheduled work.
            h.root
                .try_issue_background_semantic_pass()?
                .run(|_, _| Box::pin(async { Ok(()) }))
                .await?;
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

/// Every step refreshes the session's PvP locks from durable state first; a step that met a
/// background pass on the holder was rejected (the node-boot use-wire `step_blocked` stage).
#[test]
fn a_step_lock_refresh_waits_for_a_busy_holder_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("pvp-refresh-busy-holder", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "pvp_refresh_busy_holder", true, false)
                    .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            dependencies.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            track(&owner, &dependencies)?;
            let session = dependencies.admitted.game_session_id;
            assert!(owner.refresh_pvp_locks(dependencies.actor, session).await);
            assert!(
                holder_return_in_final_retry_interval(
                    &h.root,
                    owner.refresh_pvp_locks(dependencies.actor, session),
                )
                .await?,
                "the PvP read refused before the holder deadline"
            );
            let refreshed = priority_handoff(
                &h.root,
                owner.refresh_pvp_locks(dependencies.actor, session),
            )
            .await?;
            assert!(refreshed, "the refresh refused on a busy holder");
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn quest_callback_resume_waits_for_the_grizzly_lookup_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("quest-callback-busy-holder", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "quest_callback_busy_holder",
                true,
                false,
            )
            .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            let mut admitted = dependencies.admitted;
            let busy = hold_holder(&h.root, Duration::from_millis(1_250))?;

            let playable = owner.prepare_quest_callback_resume(&mut admitted).await;

            busy.await??;
            assert!(playable, "a returning holder must allow the first Grizzly visit lookup");
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn activity_input_waits_for_a_busy_holder_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("activity-input-busy-holder", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "activity_input_busy_holder",
                true,
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
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            let mut admitted = dependencies.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(
                FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0,
            );
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    &[crate::gameplay_transport::capabilities::OfferedCapability {
                        id: oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
                        requires: &[],
                    }],
                    &[oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1],
                )
                .ok_or("activity capability")?;
            assert!(owner.activity_transfer_input(admitted).await.is_ok());
            assert!(
                holder_return_in_final_retry_interval(&h.root, async {
                    owner.activity_transfer_input(admitted).await.is_ok()
                })
                .await?,
                "the activity input refused before the holder deadline"
            );
            let accepted = priority_handoff(&h.root, async {
                owner.activity_transfer_input(admitted).await.is_ok()
            })
            .await?;
            assert!(
                accepted,
                "a background pass must not close ordinary activity-capable input"
            );
            // A persistently unavailable root still authorizes no input.
            let held = h.root.try_issue_semantic_pass()?;
            assert!(matches!(
                owner.activity_transfer_input(admitted).await,
                Err(crate::durability::DurabilityError::RootUnavailable)
            ));
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn committed_first_entry_does_not_wait_for_unrelated_admission_writer_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("committed-entry-read", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "committed_entry_read", true, false).await?;
            let request = prepare_current_session(&h).await?;
            let store = FreshAdmissionStore::from_root(h.root.clone());
            let mut other = h.pool.begin().await?;
            sqlx::query("LOCK TABLE game_durability_admission_account_guards IN ROW EXCLUSIVE MODE")
                .execute(&mut *other).await?;
            assert!(matches!(
                tokio::time::timeout(
                    Duration::from_secs(1),
                    store.observe_committed_admission(request.operation()),
                ).await??,
                crate::durability::fresh_admission::FreshReconciliation::Committed(_)
            ));
            other.rollback().await?;
            h.cleanup().await
        })
    })
}

#[test]
fn promotion_read_ignores_unrelated_admission_writer_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("promotion-observation", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async move {
                let h = Harness::create_with_admission_seed(admin, "promotion_observation", true, false).await?;
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let session = Dependencies::new()?.admitted.game_session_id;
                let store = FreshAdmissionStore::from_root(h.root.clone());
                let fence = crate::gameplay_transport::item_fence_of(store.current_session_at(session).await?.0)
                    .ok_or("promotion item fence")?;
                let mut writer = h.pool.begin().await?;
                sqlx::query("LOCK TABLE game_durability_admission_account_guards IN ROW EXCLUSIVE MODE")
                    .execute(&mut *writer)
                    .await?;
                assert!(h.root.is_ready());
                assert!(h.root.checked_out_semantic_pass().is_none());
                assert!(!tokio::time::timeout(
                    Duration::from_secs(1),
                    h.root.read_promotion_for_session(&authority, &h.node, fence),
                ).await??);
                writer.rollback().await?;
                drop(authority);
                drop(seal);
                h.cleanup().await
            })
    })
}

// Poll between advances so the retry reaches the final partial interval while
// PostgreSQL's only holder is still checked out. Resume time before doing SQL.
async fn holder_return_in_final_retry_interval(
    root: &crate::durability::DurabilityRoot,
    operation: impl Future<Output = bool>,
) -> TestResult<bool> {
    use std::task::Poll;
    let held = root.issue_ready_semantic_pass().await?;
    let mut operation = std::pin::pin!(
        crate::durability::DurabilityRoot::with_holder_owner(operation)
    );
    tokio::time::pause();
    let mut result = std::future::poll_fn(|cx| Poll::Ready(operation.as_mut().poll(cx))).await;
    for _ in 0..4 {
        if result.is_ready() {
            break;
        }
        // Advance slightly past the timer's millisecond resolution.
        tokio::time::advance(Duration::from_millis(201)).await;
        result = std::future::poll_fn(|cx| Poll::Ready(operation.as_mut().poll(cx))).await;
    }
    tokio::time::resume();
    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
    match result {
        Poll::Pending => Ok(operation.await),
        Poll::Ready(result) => Ok(result),
    }
}

// The caller is the real fenced read/input guard. Keep its holder identity independent
// from the fixture returning the pass, then reproduce immediate scheduler reacquisition.
async fn priority_handoff(
    root: &crate::durability::DurabilityRoot,
    operation: impl std::future::Future<Output = bool>,
) -> TestResult<bool> {
    use std::{future::Future, task::Poll};
    let held = root.try_issue_background_semantic_pass().map_err(debug)?;
    let operation = crate::durability::DurabilityRoot::with_holder_owner(operation);
    let mut operation = std::pin::pin!(operation);
    let first = std::future::poll_fn(|cx| Poll::Ready(operation.as_mut().poll(cx))).await;
    assert!(first.is_pending(), "a busy guard must await the holder");
    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
    for _ in 0..16 {
        assert!(matches!(
            root.try_issue_background_semantic_pass(),
            Err(crate::durability::DurabilityError::RootUnavailable)
        ));
    }
    let result = operation.await;
    root.try_issue_background_semantic_pass()?
        .run(|_, _| Box::pin(async { Ok(()) }))
        .await?;
    Ok(result)
}

fn hold_holder(
    root: &crate::durability::DurabilityRoot,
    hold: Duration,
) -> TestResult<tokio::task::JoinHandle<Result<(), crate::durability::DurabilityError>>> {
    let pass = root.try_issue_semantic_pass().map_err(debug)?;
    Ok(tokio::spawn(pass.run(move |_holder, _deadline| {
        Box::pin(async move {
            tokio::time::sleep(hold).await;
            Ok(())
        })
    })))
}


#[test]
fn ground_projection_waits_for_a_busy_holder_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("ground-projection-busy-holder", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "ground_projection_busy", true, false).await?;
            let fence = super::fixture::fence(1)?;
            let read = crate::gameplay_transport::loop_move::projection_read(
                &h.root,
                async || h.root.read_item_loop_ground_for_scope(fence.runtime_scope).await,
                crate::gameplay_transport::root_holder_busy,
            );
            assert!(priority_handoff(&h.root, async { read.await.is_some() }).await?);
            let attempts = std::cell::Cell::new(0);
            let rejected = crate::gameplay_transport::loop_move::projection_read(
                &h.root,
                async || {
                    attempts.set(attempts.get() + 1);
                    Err::<(), _>(crate::durability::DurabilityError::CommitRejected)
                },
                crate::gameplay_transport::root_holder_busy,
            ).await;
            assert!(rejected.is_none());
            assert_eq!(attempts.get(), 1);
            h.cleanup().await
        })
    })
}
