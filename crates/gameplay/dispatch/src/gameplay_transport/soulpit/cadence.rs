//! Soulpit uses the existing world owner, independently of connected player loops.
use super::super::{ComposedFreshAdmission, world_respawns::stopping_first};
use super::handoff_owner::{READ_RETRY_MICROS, SoulpitWake};
use crate::monster_owner_cycle::respawn_job::RespawnExit;
use oteryn_foundation::CancellationToken;
use std::time::Duration;
use tokio::time::Instant;

async fn wait_for_discovery_retry(
    deadline: Instant,
    ready: impl Fn() -> bool,
    mut wait_for_holder: impl AsyncFnMut(Instant, Instant),
) {
    loop {
        let now = Instant::now();
        if now >= deadline {
            return;
        }
        let latest = (now + Duration::from_micros(READ_RETRY_MICROS)).min(deadline);
        wait_for_holder(now, latest).await;
        if ready() {
            return;
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn run_soulpits(
        &self,
        shutdown: &CancellationToken,
    ) -> RespawnExit {
        // Crystal's global encounter tick is independent of addEvent's exact deadlines.
        let mut global_due = self.owner_now().get().saturating_add(1_000_000);
        loop {
            let now = self.owner_now().get();
            let global_tick = now >= global_due;
            if global_tick {
                global_due = now.saturating_add(1_000_000);
            }
            let wake = if let Some(owner) = self.states.boss_activity_handoff.get() {
                match stopping_first(owner.drive_soulpit(self, now, global_tick), shutdown).await {
                    None => return RespawnExit::Shutdown,
                    Some(Ok(wake)) => wake,
                    Some(Err(())) => {
                        self.runtime.lock().await.retire_owner_cycle();
                        return RespawnExit::StaleOwner;
                    }
                }
            } else {
                SoulpitWake::Scheduled(None)
            };
            if wake.scheduled_due().is_some_and(|due| due <= now) {
                self.runtime.lock().await.retire_owner_cycle();
                return RespawnExit::StaleOwner;
            }
            let (deadline, busy) = match wake {
                SoulpitWake::Scheduled(due) => (due, false),
                SoulpitWake::DiscoveryBusy { room_due, .. } => (room_due, true),
            };
            // Crystal 201189f data/scripts/globalevents/encounters.lua:12. The activity
            // owner also wakes exactly for the separate four-second spawn and ten-minute
            // kick events; only global ticks may clear a wave after its 4.5-second gate.
            let due = deadline.map_or(global_due, |deadline| deadline.min(global_due));
            let delay = Duration::from_micros(due.saturating_sub(self.owner_now().get()));
            let deadline = Instant::now() + delay;
            let idle = async {
                if busy {
                    // The discovery grant and transition guard are gone. Readiness is
                    // only a hint for the next pass, which must obtain its own grant.
                    wait_for_discovery_retry(
                        deadline,
                        || self.root.background_holder_ready(),
                        async |earliest, latest| {
                            self.root.wait_holder_retry(earliest, latest).await
                        },
                    )
                    .await;
                } else {
                    tokio::time::sleep_until(deadline).await;
                }
            };
            if stopping_first(idle, shutdown).await.is_none() {
                return RespawnExit::Shutdown;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use tokio::sync::Notify;

    #[test]
    fn busy_discovery_retries_once_after_holder_return_or_ready_backoff()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .start_paused(true)
            .build()?;
        runtime.block_on(async {
            for notify in [true, false] {
                let returned = Notify::new();
                let ready = Cell::new(false);
                let attempts = Cell::new(0);
                let start = Instant::now();
                let backoff = Duration::from_micros(READ_RETRY_MICROS);
                let retry = async {
                    attempts.set(attempts.get() + 1);
                    wait_for_discovery_retry(
                        start + 5 * backoff,
                        || ready.get(),
                        async |_, latest| {
                            let _ = tokio::time::timeout_at(latest, returned.notified()).await;
                        },
                    )
                    .await;
                    attempts.set(attempts.get() + 1);
                    Instant::now()
                };
                let release = async {
                    tokio::time::sleep_until(start + 3 * backoff + backoff / 2).await;
                    assert_eq!(attempts.get(), 1, "busy backoffs cannot issue new passes");
                    ready.set(true);
                    if notify {
                        returned.notify_waiters();
                    }
                };
                let (retried_at, ()) = tokio::join!(retry, release);
                assert_eq!(attempts.get(), 2);
                let expected = if notify {
                    3 * backoff + backoff / 2
                } else {
                    4 * backoff
                };
                assert!(
                    retried_at >= start + expected && retried_at < start + expected + backoff / 2
                );
            }
        });
        Ok(())
    }
}
