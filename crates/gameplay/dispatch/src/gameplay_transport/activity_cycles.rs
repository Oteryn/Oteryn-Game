//! Poll each live Instance's existing autonomous owners alongside the shared
//! handoff/deadline pass. SQL in one pass never stops another room's native AI.
use super::{ComposedFreshAdmission, activity_runtime::ScopeKey};
use oteryn_foundation::CancellationToken;
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::task::Poll;

type ActivityCycleFuture<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;
type ActivityCycleJob<'a> = (ScopeKey, Arc<CancellationToken>, ActivityCycleFuture<'a>);
type ActivityRecoveryJob<'a> = (Vec<u8>, ActivityCycleFuture<'a>);

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn run_activity_owner_cycles(&self, shutdown: &CancellationToken) {
        let mut jobs: Vec<ActivityCycleJob<'_>> = Vec::new();
        let mut recoveries: Vec<ActivityRecoveryJob<'_>> = Vec::new();
        loop {
            let owners = self.activity_runtimes.activity_owners().unwrap_or_default();
            for (key, stop, _) in &jobs {
                if shutdown.is_cancelled()
                    || !owners
                        .iter()
                        .any(|owner| ScopeKey::new(owner.scope, owner.generation) == *key)
                {
                    stop.cancel();
                }
            }
            if !shutdown.is_cancelled() {
                for (key, pending) in self.abandoned_activity_recoveries() {
                    if recoveries.iter().any(|(present, _)| *present == key) {
                        continue;
                    }
                    let task = Box::pin(async move {
                        let _ = super::first(
                            self.reconcile_abandoned_activity_recovery(pending),
                            shutdown.cancelled(),
                        )
                        .await;
                    });
                    recoveries.push((key, task));
                }
                for owner in owners {
                    let key = ScopeKey::new(owner.scope, owner.generation);
                    if jobs.iter().any(|(existing, _, _)| *existing == key) {
                        continue;
                    }
                    // Discovery must not block polling existing rooms while a
                    // different handoff temporarily owns its native mutex.
                    let inhabited = owner.runtime.try_lock().is_ok_and(|runtime| {
                        runtime
                            .activity_participant_sessions()
                            .is_ok_and(|sessions| !sessions.is_empty())
                    });
                    if !inhabited {
                        continue;
                    }
                    let view = self.runtime_view(&owner);
                    let stop = Arc::new(CancellationToken::new());
                    let running = Arc::clone(&stop);
                    // Each method retains its existing clocks/deadlines. The
                    // heartbeat below only discovers newly occupied scopes.
                    let task = Box::pin(async move {
                        let _ = super::first(
                            view.run_source_owner_cycles(&running),
                            view.run_last_position_writes(&running),
                        )
                        .await;
                    });
                    jobs.push((key, stop, task));
                }
            }
            let mut pass = pin!(async {
                let _ = self.activity_lifecycle_pass().await;
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            });
            let mut stopping = pin!(shutdown.cancelled());
            let mut stopped = shutdown.is_cancelled();
            let mut pass_done = false;
            poll_fn(|context| {
                if stopping.as_mut().poll(context).is_ready() {
                    stopped = true;
                }
                if stopped {
                    for (_, stop, _) in &jobs {
                        stop.cancel();
                    }
                }
                jobs.retain_mut(|(_, _, job)| job.as_mut().poll(context).is_pending());
                recoveries.retain_mut(|(_, job)| job.as_mut().poll(context).is_pending());
                if !pass_done && pass.as_mut().poll(context).is_ready() {
                    pass_done = true;
                }
                if pass_done && (!stopped || (jobs.is_empty() && recoveries.is_empty())) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
            if stopped {
                return;
            }
        }
    }
}
