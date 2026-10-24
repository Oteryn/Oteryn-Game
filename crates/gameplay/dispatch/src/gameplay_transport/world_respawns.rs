//! Wake/mutex composition for the World-owned, session-independent respawn job.
use super::ComposedFreshAdmission;
use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros};
use crate::monster_owner_cycle::respawn_job::{MonsterRespawnJob, RespawnExit};
use oteryn_foundation::CancellationToken;

pub(super) trait RespawnWake: OwnerClock {
    fn sleep_until(&self, due: SemanticTimeMicros) -> impl Future<Output = ()> + Send;
}

pub(super) struct MonotonicRespawnWake {
    origin: std::time::Instant,
}
impl MonotonicRespawnWake {
    pub(super) fn from_origin(origin: std::time::Instant) -> Self {
        Self { origin }
    }
}
impl OwnerClock for MonotonicRespawnWake {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(
            u64::try_from(self.origin.elapsed().as_micros()).unwrap_or(u64::MAX),
        )
    }
}
impl RespawnWake for MonotonicRespawnWake {
    async fn sleep_until(&self, due: SemanticTimeMicros) {
        let delay = due.get().saturating_sub(self.now().get());
        tokio::time::sleep(std::time::Duration::from_micros(delay)).await;
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn run_world_respawns(&self, shutdown: &CancellationToken) -> RespawnExit {
        // A busy durability holder is transient; never stop the world's existing spawn loop.
        loop {
            let Some(initialized) =
                stopping_first(self.initialize_cyclopedia_respawn(), shutdown).await
            else {
                return RespawnExit::Shutdown;
            };
            if initialized.is_ok() {
                break;
            }
            if stopping_first(
                tokio::time::sleep(std::time::Duration::from_secs(1)),
                shutdown,
            )
            .await
            .is_none()
            {
                return RespawnExit::Shutdown;
            }
        }
        drive(
            self.runtime,
            &MonotonicRespawnWake::from_origin(self.clock_origin),
            shutdown,
        )
        .await
    }
}

pub(super) async fn drive(
    runtime: &tokio::sync::Mutex<crate::foundation::ChannelRuntimeV1>,
    wake: &impl RespawnWake,
    shutdown: &CancellationToken,
) -> RespawnExit {
    let mut job = None;
    loop {
        // Poll shutdown first, including while waiting for the runtime mutex.
        let pass = async {
            let mut runtime = runtime.lock().await;
            if shutdown.is_cancelled() {
                return Err(RespawnExit::Shutdown);
            }
            if job.is_none() {
                job = Some(MonsterRespawnJob::new(&runtime)?);
            }
            let Some(job) = job.as_mut() else {
                return Err(RespawnExit::StaleOwner);
            };
            let now = wake.now().get();
            job.pass(&mut runtime, now)?;
            job.next_owner_window(now)
                .ok_or(RespawnExit::ClockExhausted)
        };
        let Some(result) = stopping_first(pass, shutdown).await else {
            return RespawnExit::Shutdown;
        };
        let due = match result {
            Ok(due) => due,
            Err(exit) => return exit,
        };
        if stopping_first(
            wake.sleep_until(SemanticTimeMicros::from_micros(due)),
            shutdown,
        )
        .await
        .is_none()
        {
            return RespawnExit::Shutdown;
        }
    }
}

#[cfg(test)]
mod world_respawns_tests;

async fn stopping_first<T>(
    work: impl Future<Output = T>,
    shutdown: &CancellationToken,
) -> Option<T> {
    let mut stop = std::pin::pin!(shutdown.cancelled());
    let mut work = std::pin::pin!(work);
    std::future::poll_fn(|cx| {
        if stop.as_mut().poll(cx).is_ready() {
            return std::task::Poll::Ready(None);
        }
        work.as_mut().poll(cx).map(Some)
    })
    .await
}
