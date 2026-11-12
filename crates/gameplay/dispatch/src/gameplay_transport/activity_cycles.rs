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
        let mut durable_scan = ActivityDurableScan::new(tokio::time::Instant::now());
        loop {
            let owners = self.activity_runtimes.activity_owners().unwrap_or_default();
            for (key, stop, _) in &jobs {
                if shutdown.is_cancelled()
                    || owners.iter().any(|owner| {
                        ScopeKey::new(owner.scope, owner.generation) == *key
                            && owner.states.kill_source_owner.stop_requested()
                    })
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
                    if !owner.states.kill_source_owner.start() {
                        continue;
                    }
                    let view = self.runtime_view(&owner);
                    let stop = Arc::new(CancellationToken::new());
                    let running = Arc::clone(&stop);
                    // Each method retains its existing clocks/deadlines. The
                    // heartbeat below only discovers newly occupied scopes.
                    let task = Box::pin(async move {
                        let _ = super::first(
                            super::first(
                                view.run_source_owner_cycles(&running),
                                view.run_last_position_writes(&running),
                            ),
                            view.states.kill_source_owner.cancelled(),
                        )
                        .await;
                        // first() has dropped both child futures before the join
                        // acknowledgement; a terminal snapshot cannot race them.
                        view.states.kill_source_owner.complete();
                    });
                    jobs.push((key, stop, task));
                }
            }
            let mut pass = pin!(async {
                let scan = durable_scan.due(tokio::time::Instant::now());
                let _ = self.activity_lifecycle_pass_with(scan).await;
                let _ = self.publish_activity_raid_presence().await;
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            });
            let mut stopping = pin!(shutdown.cancelled());
            let mut stopped = shutdown.is_cancelled();
            let mut pass_done = false;
            poll_fn(|context| {
                // `cancelled()` panics if polled again after it completed, and this pass keeps
                // polling while stopped jobs drain.
                if !stopped && stopping.as_mut().poll(context).is_ready() {
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

/// Interval between the lifecycle's ownerless database traversals (NBHOG-1).
pub(super) const ACTIVITY_DURABLE_SCAN_CADENCE: std::time::Duration =
    std::time::Duration::from_secs(1);

/// The heartbeat polls rooms every 50 ms; the departed-recovery and due-room traversals lock every
/// admission relation, so they run at most once per [`ACTIVITY_DURABLE_SCAN_CADENCE`], the first
/// heartbeat included.
pub(super) struct ActivityDurableScan {
    next: tokio::time::Instant,
}

impl ActivityDurableScan {
    pub(super) const fn new(now: tokio::time::Instant) -> Self {
        Self { next: now }
    }

    pub(super) fn due(&mut self, now: tokio::time::Instant) -> bool {
        if now < self.next {
            return false;
        }
        self.next = now + ACTIVITY_DURABLE_SCAN_CADENCE;
        true
    }
}

#[cfg(test)]
mod durable_scan_tests {
    use super::{ACTIVITY_DURABLE_SCAN_CADENCE, ActivityDurableScan};
    use std::time::Duration;

    #[test]
    fn heartbeats_run_the_durable_scan_once_per_cadence() {
        let start = tokio::time::Instant::now();
        let mut scan = ActivityDurableScan::new(start);
        // Two seconds of 50 ms heartbeats, as `run_activity_owner_cycles` polls.
        let runs = (0..40_u32)
            .filter(|tick| scan.due(start + Duration::from_millis(50) * *tick))
            .count();
        assert_eq!(runs, 2);
        assert!(scan.due(start + ACTIVITY_DURABLE_SCAN_CADENCE * 2));
    }

    #[test]
    fn the_first_heartbeat_scans() {
        let start = tokio::time::Instant::now();
        let mut scan = ActivityDurableScan::new(start);
        assert!(scan.due(start));
        assert!(!scan.due(start + Duration::from_millis(50)));
    }
}
