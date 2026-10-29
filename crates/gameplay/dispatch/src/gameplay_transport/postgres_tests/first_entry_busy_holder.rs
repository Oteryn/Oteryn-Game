//! NODEBOOT-1: the first entry of a committed actor waits for a busy durability holder
//! instead of leaving the admitted session not playable.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::AuthenticatedTransportRefV1;
use crate::gameplay_transport::AdmissionRetryBudget;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use std::time::Duration;

#[test]
fn a_committed_actor_enters_after_a_busy_holder_is_returned_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("first-entry-busy-holder", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "first_entry_busy_holder",
                false,
                false,
            )
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
            Ok(())
        })
    })
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
