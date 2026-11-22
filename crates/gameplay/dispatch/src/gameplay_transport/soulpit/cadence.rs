//! Soulpit uses the existing world owner, independently of connected player loops.
use super::super::{ComposedFreshAdmission, world_respawns::stopping_first};
use super::handoff_owner::{READ_RETRY_MICROS, SoulpitWake};
use crate::monster_owner_cycle::respawn_job::RespawnExit;
use oteryn_foundation::CancellationToken;
use std::time::Duration;
use tokio::time::Instant;

async fn wait_for_discovery_retry<F: Future<Output = ()>>(
    deadline: Instant,
    ready: impl Fn() -> bool,
    holder_returned: impl Fn() -> F,
) {
    loop {
        let now = Instant::now();
        if now >= deadline {
            return;
        }
        let latest = (now + Duration::from_micros(READ_RETRY_MICROS)).min(deadline);
        // Subscribe before sampling readiness so a return on either side of
        // the subscription is visible to this retry.
        let returned = holder_returned();
        if ready() {
            return;
        }
        let _ = tokio::time::timeout_at(latest, returned).await;
    }
}

async fn wait_for_wake<F: Future<Output = ()>>(
    wake: SoulpitWake,
    global_due: u64,
    now_micros: u64,
    ready: impl Fn() -> bool,
    holder_returned: impl Fn() -> F,
) {
    let due = wake
        .scheduled_due()
        .map_or(global_due, |due| due.min(global_due));
    let deadline = Instant::now() + Duration::from_micros(due.saturating_sub(now_micros));
    if matches!(wake, SoulpitWake::DiscoveryBusy { .. }) {
        // Readiness is a hint. The next pass still has to obtain its own grant.
        wait_for_discovery_retry(deadline, ready, holder_returned).await;
    } else {
        tokio::time::sleep_until(deadline).await;
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
            // Crystal 201189f data/scripts/globalevents/encounters.lua:12. The activity
            // owner also wakes exactly for the separate four-second spawn and ten-minute
            // kick events; only global ticks may clear a wave after its 4.5-second gate.
            let idle = wait_for_wake(
                wake,
                global_due,
                self.owner_now().get(),
                || self.root.background_holder_ready(),
                || self.root.holder_return_notification(),
            );
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

    #[tokio::test(start_paused = true)]
    async fn busy_discovery_without_rooms_retries_at_the_next_backoff() {
        let start = Instant::now();
        let returned = Notify::new();
        wait_for_wake(
            SoulpitWake::DiscoveryBusy {
                room_due: None,
                refused_at: 0,
            },
            1_000_000,
            0,
            || false,
            || returned.notified(),
        )
        .await;
        let backoff = Duration::from_micros(READ_RETRY_MICROS);
        assert!(start.elapsed() >= backoff && start.elapsed() < backoff + Duration::from_millis(2));
    }

    #[tokio::test(start_paused = true)]
    async fn busy_discovery_retries_immediately_if_holder_returned_before_wait() {
        let start = Instant::now();
        let returned = Notify::new();
        let wake = SoulpitWake::DiscoveryBusy {
            room_due: None,
            refused_at: 0,
        };
        tokio::time::advance(Duration::from_millis(4)).await;
        returned.notify_waiters();
        let returned_at = Instant::now();

        wait_for_wake(wake, 1_000_000, 4_000, || true, || returned.notified()).await;

        assert_eq!(Instant::now(), returned_at);
        assert_eq!(start.elapsed(), Duration::from_millis(4));
    }

    #[tokio::test(start_paused = true)]
    async fn busy_discovery_catches_return_during_first_readiness_sample() {
        let start = Instant::now();
        let returned = Notify::new();
        let sampled = Cell::new(false);

        wait_for_discovery_retry(
            start + Duration::from_micros(2 * READ_RETRY_MICROS),
            || {
                if sampled.replace(true) {
                    return true;
                }
                // The holder returns after the first sample saw it busy, before
                // the notification future is first polled.
                returned.notify_waiters();
                false
            },
            || returned.notified(),
        )
        .await;

        assert_eq!(start.elapsed(), Duration::ZERO);
    }

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
                        || returned.notified(),
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
