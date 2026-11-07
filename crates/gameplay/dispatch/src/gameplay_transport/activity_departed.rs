//! Complete a departed Instance through its current origin Channel owner.
//! A missing native directory is never proof of loss. The durable activity
//! owner supplies that proof; ordinary fresh admission recreates playable state.
use super::connection::ConnectionIdentifiers;
use super::{ComposedFreshAdmission, SecureIdentifiers};
use crate::durability::activity_handoff::{
    ActivityHandoffOwner, ActivityReturnOutcome, ActivityReturnReason, ActivityReturnRequest,
};
use crate::durability::fresh_admission::ExpiredLossReleaseV1;
use crate::durability::spell_owner_commit::SpellLane;
use crate::foundation::RuntimeScopeRefV1;

#[cfg(test)]
#[derive(Default)]
pub(super) struct DepartedReturnCommittedTestBarrier {
    pub(super) entered: tokio::sync::Notify,
    pub(super) proceed: tokio::sync::Notify,
}
#[cfg(test)]
tokio::task_local! {
    pub(super) static DEPARTED_RETURN_COMMITTED_TEST_BARRIER: std::sync::Arc<DepartedReturnCommittedTestBarrier>;
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called by the origin's already running lifecycle even when this process
    /// has never registered an Instance or any previous connection.
    pub(super) async fn recover_departed_activity_pass(&self) -> Result<(), ()> {
        let (origin, generation) = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            (
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation(),
            )
        };
        if !matches!(origin, RuntimeScopeRefV1::Channel { .. }) {
            return Ok(());
        }
        let owner = ActivityHandoffOwner::new(self.root.clone());
        let mut after = None;
        let mut deferred = false;
        loop {
            let page = owner
                .departed_activities_after(self.character, self.holder, origin, generation, after)
                .await
                .map_err(|_| ())?;
            after = page.next_after;
            for activity in page.activities {
                let source = activity.source();
                // A retained native owner must first reconcile its own captured
                // effects/tokens. This path handles state lost with the process.
                let _transition = self.activity_runtimes.transitions.lock().await;
                if self
                    .activity_runtimes
                    .get(
                        source.runtime_scope,
                        source.scope_ownership_generation.get(),
                    )
                    .is_some()
                    || self.pending_activity_recovery(source.game_session_id)
                {
                    continue;
                }
                if self
                    .evidence
                    .refresh_account(self.root, self.holder, activity.account())
                    .await
                    .is_err()
                {
                    deferred = true;
                    continue;
                }
                if let Some(receipt) = activity.returned() {
                    if self
                        .activity_runtimes
                        .connection(receipt.destination_game_session_id)
                        .is_some()
                    {
                        continue;
                    }
                    // The admission-serialized discovery has resolved any sent
                    // COMMIT. It found the immutable return before discarding
                    // our retry proposal, including after a process restart.
                    self.activity_runtimes
                        .departed_recoveries
                        .lock()
                        .map_err(|_| ())?
                        .remove(&source.game_session_id);
                    match owner
                        .release_departed_return(self.character, self.holder, &activity)
                        .await
                    {
                        Ok(
                            ExpiredLossReleaseV1::Released { .. }
                            | ExpiredLossReleaseV1::Terminal
                            | ExpiredLossReleaseV1::NotExpired { .. },
                        ) => {}
                        Ok(ExpiredLossReleaseV1::NotApplicable) | Err(_) => deferred = true,
                    }
                    continue;
                }
                let request = {
                    let mut pending = self
                        .activity_runtimes
                        .departed_recoveries
                        .lock()
                        .map_err(|_| ())?;
                    if let Some(request) = pending.get(&source.game_session_id) {
                        if request.source != source || request.handoff_id != activity.handoff_id() {
                            deferred = true;
                            continue;
                        }
                        request.clone()
                    } else {
                        let request = ActivityReturnRequest {
                            return_id: *SecureIdentifiers.game_session_id().ok_or(())?.as_bytes(),
                            handoff_id: activity.handoff_id(),
                            command_id: None,
                            source,
                            destination_game_session_id: SecureIdentifiers
                                .game_session_id()
                                .ok_or(())?,
                            destination_transport: None,
                            destination_generation: activity.origin_generation(),
                            reason: ActivityReturnReason::OwnerLost,
                            current_vitals: None,
                            current_rest: None,
                        };
                        pending.try_reserve(1).map_err(|_| ())?;
                        pending.insert(source.game_session_id, request.clone());
                        request
                    }
                };
                let lane = SpellLane::for_scope(source.runtime_scope);
                let permit = lane.acquire().await.map_err(|_| ())?;
                let outcome = owner
                    .recover_departed_death(&permit, self.character, self.holder, request)
                    .await;
                #[cfg(test)]
                if matches!(&outcome, Ok(ActivityReturnOutcome::Returned(_)))
                    && let Ok(barrier) =
                        DEPARTED_RETURN_COMMITTED_TEST_BARRIER.try_with(std::sync::Arc::clone)
                {
                    barrier.entered.notify_one();
                    barrier.proceed.notified().await;
                }
                match outcome {
                    Ok(ActivityReturnOutcome::Returned(_) | ActivityReturnOutcome::Replayed(_)) => {
                        self.activity_runtimes
                            .departed_recoveries
                            .lock()
                            .map_err(|_| ())?
                            .remove(&source.game_session_id);
                    }
                    Ok(ActivityReturnOutcome::Refused(_)) => {
                        // A proved refusal committed no return. The next current
                        // discovery may retry an updated origin allocation.
                        self.activity_runtimes
                            .departed_recoveries
                            .lock()
                            .map_err(|_| ())?
                            .remove(&source.game_session_id);
                        deferred = true;
                    }
                    Err(_) => deferred = true, // retain the exact unknown proposal
                }
            }
            if after.is_none() {
                break;
            }
        }

        let mut after = None;
        loop {
            let (activities, next) = owner
                .departed_retirements_after(self.character, self.holder, origin, generation, after)
                .await
                .map_err(|_| ())?;
            after = next;
            for activity in activities {
                let _transition = self.activity_runtimes.transitions.lock().await;
                if self
                    .activity_runtimes
                    .get(activity.scope(), activity.generation().get())
                    .is_some()
                {
                    continue;
                }
                let lane = SpellLane::for_scope(activity.scope());
                let permit = lane.acquire().await.map_err(|_| ())?;
                // The same one-item reservation/receipt/audit owner handles
                // private Ground; no native owner or control grant is created.
                if self
                    .root
                    .retire_activity_ground_step(&permit, self.character, self.holder, &activity)
                    .await
                    .is_err()
                {
                    deferred = true;
                }
            }
            if after.is_none() {
                break;
            }
        }
        if deferred { Err(()) } else { Ok(()) }
    }
}
