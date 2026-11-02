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
            let Some(initialized) = stopping_first(
                async {
                    self.initialize_daily_boost().await?;
                    self.initialize_cyclopedia_respawn().await
                },
                shutdown,
            )
            .await
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
        let ordinary = async {
            drive(
                &self.runtime,
                &MonotonicRespawnWake::from_origin(self.clock_origin),
                shutdown,
            )
            .await
        };
        tokio::pin!(ordinary);
        let bosses = self.run_boss_spawns(shutdown);
        tokio::pin!(bosses);
        let echoes = self.run_echo_wardens(shutdown);
        tokio::pin!(echoes);
        let soulpits = self.run_soulpits(shutdown);
        tokio::pin!(soulpits);
        let raids = self.run_raid_effects(shutdown);
        tokio::pin!(raids);
        std::future::poll_fn(|cx| {
            if let std::task::Poll::Ready(exit) = raids.as_mut().poll(cx) {
                return std::task::Poll::Ready(exit);
            }
            if let std::task::Poll::Ready(exit) = soulpits.as_mut().poll(cx) {
                return std::task::Poll::Ready(exit);
            }
            if let std::task::Poll::Ready(exit) = echoes.as_mut().poll(cx) {
                return std::task::Poll::Ready(exit);
            }
            if let std::task::Poll::Ready(exit) = bosses.as_mut().poll(cx) {
                return std::task::Poll::Ready(exit);
            }
            ordinary.as_mut().poll(cx)
        })
        .await
    }

    pub(super) async fn boss_spawn_pass(
        &self,
        timers: &mut std::collections::BTreeMap<String, Option<tokio::time::Instant>>,
    ) -> Result<bool, RespawnExit> {
        use crate::durability::boss_encounter::{BossClockClaim, BossClockError, BossClockKey};
        let mut checked_all = true;
        let (binding, spawns) = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| RespawnExit::StaleOwner)?;
            (runtime.binding(), runtime.boss_spawns().to_vec())
        };
        // Lever rooms own a one-shot boss; the open-world spawn clock remains Channel-only.
        let Some(channel) = binding.channel_id() else {
            return Ok(true);
        };
        for spawn in spawns {
            if timers
                .get(&spawn.key)
                .is_some_and(|due| due.is_none_or(|due| due > tokio::time::Instant::now()))
            {
                continue;
            }
            if let Some(actor) = spawn.actor {
                let runtime = self.runtime.lock().await;
                if runtime.companion_snapshot(actor).is_ok() {
                    continue;
                }
            }
            if spawn.pending_claim {
                let mut runtime = self.runtime.lock().await;
                if runtime.binding() != binding
                    || runtime.realize_boss_spawn(&spawn.key, spawn.cycle).is_err()
                {
                    runtime.retire_owner_cycle();
                    return Err(RespawnExit::StaleOwner);
                }
                continue;
            }
            let key = BossClockKey {
                world: binding.world_id(),
                channel,
                spawn: spawn.key.clone(),
            };
            let outcome = self
                .root
                .activate_boss_clock(
                    self.character,
                    self.holder,
                    key,
                    binding.scope_generation(),
                    spawn.policy,
                )
                .await;
            match outcome {
                Ok(BossClockClaim::Claimed(clock)) => {
                    let mut runtime = self.runtime.lock().await;
                    if runtime.binding() != binding
                        || runtime.realize_boss_spawn(&spawn.key, clock.cycle).is_err()
                    {
                        runtime.retire_owner_cycle();
                        return Err(RespawnExit::StaleOwner);
                    }
                }
                Ok(BossClockClaim::Waiting(clock)) => {
                    let due = clock.due_at_unix_ms.and_then(|due| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()?;
                        let delay = (due as u128).saturating_sub(now.as_millis());
                        tokio::time::Instant::now().checked_add(std::time::Duration::from_millis(
                            u64::try_from(delay).ok()?,
                        ))
                    });
                    timers.insert(spawn.key, due);
                }
                Ok(BossClockClaim::Unchanged | BossClockClaim::CompareFailed) => {}
                Err(BossClockError::Unavailable(
                    crate::durability::DurabilityError::RootUnavailable,
                )) => {
                    // This operation emits RootUnavailable at pass acquisition, before
                    // any transaction begins. Normal holder contention is safe to retry.
                    checked_all = false;
                }
                Err(_) => {
                    // A commit can be uncertain. End this owner rather than inventing
                    // a second installation from an ALIVE row of the same generation.
                    self.runtime.lock().await.retire_owner_cycle();
                    return Err(RespawnExit::StaleOwner);
                }
            }
        }
        Ok(checked_all)
    }

    async fn run_boss_spawns(&self, shutdown: &CancellationToken) -> RespawnExit {
        // Portable database deadlines become process-local owner timers. An INACTIVE
        // clock is parked for this owner; a new reset drains and replaces the owner.
        let mut timers = std::collections::BTreeMap::<String, Option<tokio::time::Instant>>::new();
        loop {
            let pass = self.boss_spawn_pass(&mut timers);
            match stopping_first(pass, shutdown).await {
                None => return RespawnExit::Shutdown,
                Some(Err(exit)) => return exit,
                Some(Ok(_)) => {}
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

pub(super) async fn stopping_first<T>(
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
