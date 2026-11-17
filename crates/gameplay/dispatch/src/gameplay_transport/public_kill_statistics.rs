//! Bounded physical-source batching, independent of reward/credited-player admission.
//! The native lethal owner freezes the canonical occurrence and catalogue class;
//! persistence is polled after runtime locks are released (PUBPROJ owner2026-11-07).
use super::ComposedFreshAdmission;
use crate::durability::public_projections::daily_kills::{
    KILL_BATCH_MAX, KILL_PENDING_MAX, PhysicalKill,
};
use crate::foundation::{ChannelRuntimeV1, CreatureDeathOccurrenceKey, ExactActorRef};
use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Default)]
pub(super) struct CaptureQueue {
    inner: Mutex<State>,
}
#[derive(Debug, Default)]
struct State {
    pending: VecDeque<PhysicalKill>,
    // Keep original UTC facts while the native dead actor or its corpse is
    // retained. This shares the finite4096 source envelope, without a new actor
    // authority or an unbounded history cache.
    retained: Vec<(ExactActorRef, PhysicalKill)>,
    prune_cursor: usize,
    finalized: bool,
    last_clock: Option<i64>,
    last_ack_cut: Option<i64>,
    last_attempt_ms: Option<i64>,
    gap_ms: Option<i64>,
}
#[derive(Debug)]
pub(super) struct Batch {
    pub events: Vec<PhysicalKill>,
    pub cut_ms: i64,
    pub gap_ms: Option<i64>,
    pub more: bool,
}
impl CaptureQueue {
    fn state(&self) -> MutexGuard<'_, State> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(super) fn source_gap(&self, now_ms: i64) {
        let mut state = self.state();
        let gap = state
            .last_clock
            .or(state.last_ack_cut)
            .unwrap_or(now_ms.max(0));
        state.gap_ms = Some(state.gap_ms.map_or(gap, |old| old.min(gap)));
    }
    pub(super) fn capture(
        &self,
        _runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        death: CreatureDeathOccurrenceKey,
        creature: &str,
        boss: bool,
        now_ms: i64,
    ) {
        let mut state = self.state();
        if state.retained.iter().any(|(actor, _)| *actor == target) {
            return;
        }
        if now_ms < 0 || state.last_clock.is_some_and(|old| now_ms < old) {
            let gap = state.last_clock.or(state.last_ack_cut).unwrap_or(0);
            state.gap_ms = Some(state.gap_ms.map_or(gap, |old| old.min(gap)));
            return;
        }
        state.last_clock = Some(now_ms);
        let Some(event) = PhysicalKill::capture(death, creature, boss, now_ms) else {
            state.gap_ms = Some(state.gap_ms.map_or(now_ms, |old| old.min(now_ms)));
            return;
        };
        if state.pending.len() >= KILL_PENDING_MAX
            || state.retained.len() >= KILL_PENDING_MAX
            || state.pending.try_reserve(1).is_err()
            || state.retained.try_reserve(1).is_err()
        {
            state.gap_ms = Some(state.gap_ms.map_or(now_ms, |old| old.min(now_ms)));
            return;
        }
        state.retained.push((target, event.clone()));
        state.pending.push_back(event);
    }
    // Autonomous maintenance only: snapshotting a companion clones its native
    // state, so never scan retained actors in the lethal combat critical path.
    fn prune_retained(&self, runtime: &ChannelRuntimeV1) {
        let mut state = self.state();
        for _ in 0..32 {
            if state.retained.is_empty() {
                break;
            }
            state.prune_cursor %= state.retained.len();
            let index = state.prune_cursor;
            let actor = state.retained[index].0;
            if runtime.retains_corpse_projection(actor)
                || runtime.companion_snapshot_including_dead(actor).is_ok()
            {
                state.prune_cursor += 1;
            } else {
                state.retained.swap_remove(index);
            }
        }
    }
    pub(super) fn batch(&self, now_ms: i64, force: bool) -> Option<Batch> {
        self.build_batch(now_ms, force, false)
    }
    /// Only serving's joined-owner boundary can provide this token. A forced
    /// polling batch is not proof that later producers cannot append.
    #[cfg(test)]
    pub(super) fn final_batch(
        &self,
        now_ms: i64,
        _: &super::serving::QuiescedKillOwners,
    ) -> Option<Batch> {
        self.build_batch(now_ms, true, true)
    }
    fn build_batch(&self, now_ms: i64, force: bool, quiesced: bool) -> Option<Batch> {
        let mut state = self.state();
        if now_ms <= 0 || state.last_clock.is_some_and(|old| now_ms < old) {
            let gap = state.last_clock.or(state.last_ack_cut).unwrap_or(0);
            state.gap_ms = Some(state.gap_ms.map_or(gap, |old| old.min(gap)));
            return None;
        }
        state.last_clock = Some(now_ms);
        if !force
            && state
                .last_attempt_ms
                .is_some_and(|last| now_ms.saturating_sub(last) < 1000)
        {
            return None;
        }
        state.last_attempt_ms = Some(now_ms);
        let more = state.pending.len() > KILL_BATCH_MAX;
        let through = if quiesced && !more {
            // This is the final complete set: no producer can append another
            // same-millisecond event after the snapshot. Include every retained
            // event before claiming ended_ms; ordinary cuts remain exclusive.
            state
                .pending
                .iter()
                .map(PhysicalKill::occurred_ms)
                .max()
                .map_or(now_ms - 1, |last| last.max(now_ms - 1))
                // A joined-owner retry after acknowledging a same-ms event
                // retains that proven cut even when its queue is now empty.
                .max(state.last_ack_cut.unwrap_or(i64::MIN))
        } else {
            state
                .pending
                .get(KILL_BATCH_MAX)
                .map_or(now_ms - 1, |next| next.occurred_ms() - 1)
        };
        if state.last_ack_cut.is_some_and(|old| through < old) {
            let gap = state.last_ack_cut?;
            state.gap_ms = Some(state.gap_ms.map_or(gap, |old| old.min(gap)));
            return None;
        }
        Some(Batch {
            events: state.pending.iter().take(KILL_BATCH_MAX).cloned().collect(),
            cut_ms: through,
            gap_ms: state.gap_ms,
            more,
        })
    }
    pub(super) fn acknowledge(&self, batch: &Batch) {
        let mut state = self.state();
        for event in &batch.events {
            if state
                .pending
                .front()
                .is_some_and(|head| head.occurrence_key() == event.occurrence_key())
            {
                state.pending.pop_front();
            }
        }
        state.last_ack_cut = Some(
            state
                .last_ack_cut
                .map_or(batch.cut_ms, |old| old.max(batch.cut_ms)),
        );
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// One finite source pass, also used by the autonomous owner so idle Worlds
    /// can supply proven zero-day coverage. Errors retain the original batch.
    pub(super) async fn flush_daily_kill_statistics(&self) -> bool {
        self.flush_daily_kill_statistics_pass(false).await
    }
    /// Call only after serving joined every physical producer and quiesced the
    /// shared input gate. Failure retains its pending facts and dirty source cut.
    pub(super) async fn finish_daily_kill_statistics(
        &self,
        _proof: &super::serving::QuiescedKillOwners,
    ) {
        for _ in 0..=KILL_PENDING_MAX / KILL_BATCH_MAX {
            if self.flush_daily_kill_statistics_pass(true).await {
                break;
            }
            tokio::task::yield_now().await;
        }
    }
    /// The lifecycle holds the Instance transition lock while checking both
    /// retirement witnesses. Its local owner group has joined before this call;
    /// no shared Channel input gate is closed and no authority is installed.
    pub(super) async fn finish_activity_daily_kills(&self) -> bool {
        if !self.states.kill_source_owner.joined()
            || !self
                .runtime
                .lock()
                .await
                .activity_retirement_empty()
                .unwrap_or(false)
        {
            return false;
        }
        for _ in 0..=KILL_PENDING_MAX / KILL_BATCH_MAX {
            if self.flush_daily_kill_statistics_pass(true).await {
                return true;
            }
            tokio::task::yield_now().await;
        }
        false
    }
    async fn flush_daily_kill_statistics_pass(&self, quiesced: bool) -> bool {
        let (binding, queue, batch) = {
            let runtime = self.runtime.lock().await;
            let queue = self.states.attack.lock().await.kills().clone();
            let now_ms = super::npc_trade_command::unix_ms_now();
            if queue.statistics().state().finalized {
                return true;
            }
            queue.statistics().prune_retained(&runtime);
            let batch = if quiesced {
                queue.statistics().build_batch(now_ms, true, true)
            } else {
                queue.statistics().batch(now_ms, false)
            };
            let Some(batch) = batch else {
                return false;
            };
            (runtime.binding(), queue, batch)
        };
        // A final source cut is possible only after every pending item fits the
        // acknowledged batch; callers quiesce input/owners before requesting it.
        let final_cut = quiesced && !batch.more;
        if self
            .root
            .flush_public_daily_kills(
                self.character,
                self.holder,
                binding.runtime_scope(),
                binding.scope_generation(),
                batch.events.clone(),
                batch.cut_ms,
                batch.gap_ms,
                final_cut,
            )
            .await
            .is_ok()
        {
            queue.statistics().acknowledge(&batch);
            if final_cut {
                queue.statistics().state().finalized = true;
            }
            !batch.more
        } else {
            false
        }
    }
}

/// One-way, per-Instance stop/join handshake. Idle -> joined also prevents a
/// newly discovered source owner from starting during terminal retirement.
#[derive(Debug, Default)]
pub(super) struct ActivityKillSourceOwner {
    state: std::sync::atomic::AtomicU8, // 0 idle, 1 running, 2 joined
    stop: oteryn_foundation::CancellationToken,
    changed: tokio::sync::Notify,
}
impl ActivityKillSourceOwner {
    pub(super) fn start(&self) -> bool {
        self.state
            .compare_exchange(
                0,
                1,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok()
    }
    pub(super) async fn cancelled(&self) {
        self.stop.cancelled().await;
    }
    pub(super) fn stop_requested(&self) -> bool {
        self.stop.is_cancelled()
    }
    pub(super) fn joined(&self) -> bool {
        self.state.load(std::sync::atomic::Ordering::Acquire) == 2
    }
    pub(super) fn complete(&self) {
        self.state.store(2, std::sync::atomic::Ordering::Release);
        self.changed.notify_waiters();
    }
    pub(super) async fn stop_and_join(&self) {
        self.stop.cancel();
        if self
            .state
            .compare_exchange(
                0,
                2,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok()
        {
            return;
        }
        loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.joined() {
                return;
            }
            notified.await;
        }
    }
}
#[cfg(test)]
mod instance_source_tests {
    use super::ActivityKillSourceOwner;
    use std::future::Future;
    #[tokio::test]
    async fn instance_final_cut_waits_for_join_and_cannot_restart_owner() {
        let owner = ActivityKillSourceOwner::default();
        assert!(owner.start());
        let mut stopping = std::pin::pin!(owner.stop_and_join());
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                stopping.as_mut().poll(cx).is_pending()
            ))
            .await
        );
        assert!(owner.stop_requested());
        assert!(!owner.joined());
        owner.complete();
        stopping.await;
        assert!(owner.joined());
        assert!(!owner.start());
    }
    #[tokio::test]
    async fn empty_instance_stop_prevents_late_owner_discovery() {
        let owner = ActivityKillSourceOwner::default();
        owner.stop_and_join().await;
        assert!(owner.joined());
        assert!(!owner.start());
    }
}
