//! Channel owner timer lane (FND-03 §10), first implemented for
//! GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1 §4.2
//! (`docs/architecture/reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md`).
//!
//! FND-03 §8.4 selects no universal fixed tick: an authoritative scope executes bounded
//! ordered work cycles, and think work, respawn work and (later) spell cooldowns/regeneration
//! all arrive as owner-scoped timer inputs, never as direct callbacks (FND-03 §10). One
//! `OwnerTimerLane` instance belongs to exactly one Channel owner for exactly one
//! `ScopeOwnershipGeneration`; a scope move creates a new lane under the new generation.
//!
//! The lane is deliberately generic over the caller's `Family` (AI think, respawn, and later
//! spell cooldown/regeneration) and `Occurrence` identity, so this stays one shared mechanism
//! rather than one bespoke scheduler per gameplay system (§4.2: "serves AI now and spell
//! cooldowns and regeneration later").
//!
//! What this module does NOT do: it never mutates game state itself. `drain_due` only returns
//! the timers whose deadline has passed, in FND-03 §10.1 deterministic order, for the owner to
//! apply as normalized inputs under its own `ScopeRuntimeFence`/`RuntimeExecutionOrdinal`. A
//! fired timer whose target no longer matches the owner's current state is dropped without
//! mutation (FND-03 §10.3) and never returned.

use super::{ExactActorRef, RuntimeExecutionOrdinal, ScopeOwnershipGeneration};

/// Process-local monotonic microsecond instant (FND-03 §8.2). It is never a durable or
/// portable timestamp and is valid only inside the owning process incarnation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticTimeMicros(u64);

impl SemanticTimeMicros {
    #[must_use]
    pub const fn from_micros(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Saturating add: a deadline computation never wraps or panics on overflow.
    #[must_use]
    pub const fn saturating_add_micros(self, delta: u64) -> Self {
        Self(self.0.saturating_add(delta))
    }
}

/// An injectable owner clock (FND-03 §11). Production wall/monotonic time and a deterministic
/// virtual clock for tests both implement it; the lane never reads a global clock directly.
pub trait OwnerClock {
    fn now(&self) -> SemanticTimeMicros;
}

/// A deterministic virtual clock for tests (FND-03 §11). Time advances only when the test
/// calls `advance`; it never moves on its own, so timer ordering and staleness tests are
/// reproducible.
#[derive(Debug)]
pub struct VirtualOwnerClock {
    now: std::cell::Cell<u64>,
}

impl VirtualOwnerClock {
    #[must_use]
    pub const fn new(start: SemanticTimeMicros) -> Self {
        Self {
            now: std::cell::Cell::new(start.0),
        }
    }

    pub fn advance(&self, delta_micros: u64) {
        self.now.set(self.now.get().saturating_add(delta_micros));
    }
}

impl OwnerClock for VirtualOwnerClock {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros(self.now.get())
    }
}

/// A rejected `schedule` call. Rejection mutates nothing (FND-03 §10.3): the caller's owner
/// state is unaffected and it decides how to react (e.g. AI-RL-06: "reject the second
/// schedule").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerTimerError {
    /// `schedule` named a different ownership generation than this lane's. The lane never
    /// re-binds to a new generation; a scope move must build a fresh lane.
    StaleOwnerGeneration,
    /// The same (family, occurrence) identity already has a pending entry (FND-03 §10.1: "the
    /// same identity is never scheduled twice").
    DuplicateOccurrence,
    /// The caller-declared pending cap for this (family, target) key is already reached (for
    /// example AI01-PENDING-TIMERS-PER-ACTOR: at most one pending think timer per creature).
    PendingLimitReached,
}

impl std::fmt::Display for OwnerTimerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::StaleOwnerGeneration => "timer schedule named a stale owner generation",
            Self::DuplicateOccurrence => "timer occurrence identity already scheduled",
            Self::PendingLimitReached => "timer pending-per-key limit reached",
        };
        formatter.write_str(text)
    }
}

impl std::error::Error for OwnerTimerError {}

/// AI-RL-06 / `AI01-PENDING-TIMERS-PER-ACTOR` (`RESOURCE_LIMITS_REGISTRY.json`): at most one
/// pending AI think timer per live creature actor. Registered by AI-1 (§4.9 of the decision);
/// enforced by the caller passing this as `max_pending_for_key` when scheduling a think timer.
pub const AI01_PENDING_TIMERS_PER_ACTOR: usize = 1;

#[derive(Debug, Clone, Copy)]
struct ScheduledTimer<Family, Occurrence> {
    family: Family,
    occurrence: Occurrence,
    target: Option<ExactActorRef>,
    due: SemanticTimeMicros,
    scheduling_ordinal: RuntimeExecutionOrdinal,
    sequence: u64,
}

/// A timer whose deadline has passed, returned by `drain_due` for the caller to apply as one
/// normalized owner input (FND-03 §10.2).
#[derive(Debug, Clone, Copy)]
pub struct FiredTimer<Family, Occurrence> {
    pub family: Family,
    pub occurrence: Occurrence,
    pub target: Option<ExactActorRef>,
    pub due: SemanticTimeMicros,
}

/// One Channel owner's timer lane for one `ScopeOwnershipGeneration` (FND-03 §10).
#[derive(Debug)]
pub struct OwnerTimerLane<Family, Occurrence> {
    generation: ScopeOwnershipGeneration,
    entries: Vec<ScheduledTimer<Family, Occurrence>>,
    next_sequence: u64,
}

impl<Family, Occurrence> OwnerTimerLane<Family, Occurrence>
where
    Family: Copy + Eq,
    Occurrence: Copy + Eq,
{
    #[must_use]
    pub const fn for_generation(generation: ScopeOwnershipGeneration) -> Self {
        Self {
            generation,
            entries: Vec::new(),
            next_sequence: 0,
        }
    }

    #[must_use]
    pub const fn generation(&self) -> ScopeOwnershipGeneration {
        self.generation
    }

    /// Number of pending timers currently occupying this exact `(family, target)` key, the
    /// same key `schedule`'s `max_pending_for_key` bounds.
    #[must_use]
    pub fn pending_for_key(&self, family: Family, target: Option<ExactActorRef>) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.family == family && entry.target == target)
            .count()
    }

    /// Schedules one owner-scoped timer (FND-03 §10.1).
    ///
    /// `scheduling_ordinal` is the `RuntimeExecutionOrdinal` of the owner resolution making
    /// this call; ties at an equal `due` order by that ordinal, then by an internal
    /// monotonically increasing sequence assigned in call order (FND-03 §10.1: "a deterministic
    /// within-resolution sequence; a separate globally visible timer counter is not required").
    ///
    /// `max_pending_for_key` bounds how many pending entries this exact `(family, target)` key
    /// may hold at once (for example `AI01_PENDING_TIMERS_PER_ACTOR`); the family-specific
    /// catch-up policy (`SKIP_TO_LATEST`, `DEADLINE_STATE`, ...) is the caller's choice of that
    /// bound, kept out of this generic lane.
    #[allow(clippy::too_many_arguments)] // Each parameter is one distinct FND-03 §10.1 key
    // component or the caller's own bound; a request struct would not shrink the call site.
    pub fn schedule(
        &mut self,
        owner_generation: ScopeOwnershipGeneration,
        scheduling_ordinal: RuntimeExecutionOrdinal,
        family: Family,
        occurrence: Occurrence,
        target: Option<ExactActorRef>,
        due: SemanticTimeMicros,
        max_pending_for_key: usize,
    ) -> Result<(), OwnerTimerError> {
        if owner_generation != self.generation {
            return Err(OwnerTimerError::StaleOwnerGeneration);
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.family == family && entry.occurrence == occurrence)
        {
            return Err(OwnerTimerError::DuplicateOccurrence);
        }
        if self.pending_for_key(family, target) >= max_pending_for_key {
            return Err(OwnerTimerError::PendingLimitReached);
        }
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.entries.push(ScheduledTimer {
            family,
            occurrence,
            target,
            due,
            scheduling_ordinal,
            sequence,
        });
        Ok(())
    }

    /// Cancels one pending occurrence, for example on scope retirement or a content revision
    /// change (§4.3). Returns whether an entry was removed.
    pub fn cancel(&mut self, family: Family, occurrence: Occurrence) -> bool {
        let before = self.entries.len();
        self.entries
            .retain(|entry| !(entry.family == family && entry.occurrence == occurrence));
        self.entries.len() != before
    }

    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.entries.len()
    }

    /// Removes and returns every timer whose deadline is at or before `clock.now()`, in
    /// deterministic order: due time, then scheduling ordinal, then within-resolution sequence
    /// (FND-03 §10.1).
    ///
    /// A stale lane (`owner_generation` no longer the lane's own generation) fires nothing
    /// (FND-03 §10.3): the caller must build a fresh lane for a new generation rather than keep
    /// scheduling into a superseded one. For each still-due entry with a `target`,
    /// `target_is_current` decides whether that target's generation still matches the owner's
    /// live state; a `false` drops the entry without mutation and without returning it,
    /// matching FND-03 §10.3's target-generation cancellation. This lane never mutates state
    /// itself: it only classifies and returns normalized inputs for the owner to apply.
    pub fn drain_due(
        &mut self,
        clock: &impl OwnerClock,
        owner_generation: ScopeOwnershipGeneration,
        mut target_is_current: impl FnMut(ExactActorRef) -> bool,
    ) -> Vec<FiredTimer<Family, Occurrence>> {
        if owner_generation != self.generation {
            return Vec::new();
        }
        let now = clock.now();
        let mut due: Vec<ScheduledTimer<Family, Occurrence>> = Vec::new();
        self.entries.retain(|entry| {
            if entry.due <= now {
                due.push(*entry);
                false
            } else {
                true
            }
        });
        due.sort_by(|left, right| {
            (left.due, left.scheduling_ordinal, left.sequence).cmp(&(
                right.due,
                right.scheduling_ordinal,
                right.sequence,
            ))
        });
        let mut fired = Vec::with_capacity(due.len());
        for entry in due {
            if let Some(target) = entry.target
                && !target_is_current(target)
            {
                continue;
            }
            fired.push(FiredTimer {
                family: entry.family,
                occurrence: entry.occurrence,
                target: entry.target,
                due: entry.due,
            });
        }
        fired
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Test-only: fixture construction and schedule() calls whose
// success is the test's own precondition, not the behavior under test.
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestFamily {
        AiThink,
        Respawn,
    }

    #[allow(clippy::expect_used)]
    fn generation(value: u64) -> ScopeOwnershipGeneration {
        ScopeOwnershipGeneration::new(value).expect("nonzero generation")
    }

    #[allow(clippy::expect_used)]
    fn ordinal(value: u64) -> RuntimeExecutionOrdinal {
        RuntimeExecutionOrdinal::new(value).expect("nonzero ordinal")
    }

    #[allow(clippy::expect_used)]
    fn uuid_v7(seed: u8) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[0] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = seed;
        bytes
    }

    #[allow(clippy::expect_used)]
    fn actor(channel_seed: u8) -> ExactActorRef {
        use crate::foundation::{ChannelId, WorldId};
        let world = WorldId::decode(&uuid_v7(1)).expect("valid UUIDv7 WorldId");
        let channel = ChannelId::decode(&uuid_v7(channel_seed)).expect("valid UUIDv7 ChannelId");
        ExactActorRef::transport_fixture(world, channel)
    }

    #[test]
    fn equal_deadline_orders_by_scheduling_ordinal_then_sequence() {
        let gen1 = generation(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
        let due = SemanticTimeMicros::from_micros(1_000);

        // Later ordinal scheduled first in call order, but must still fire after the earlier
        // ordinal at the same deadline.
        lane.schedule(gen1, ordinal(2), TestFamily::AiThink, 20, None, due, 10)
            .expect("schedule ordinal 2");
        lane.schedule(gen1, ordinal(1), TestFamily::AiThink, 10, None, due, 10)
            .expect("schedule ordinal 1");
        // Same ordinal as the first entry: must fire after it, by call-order sequence.
        lane.schedule(gen1, ordinal(1), TestFamily::AiThink, 11, None, due, 10)
            .expect("schedule ordinal 1 second");

        let clock = VirtualOwnerClock::new(due);
        let fired = lane.drain_due(&clock, gen1, |_| true);
        let occurrences: Vec<u64> = fired.iter().map(|timer| timer.occurrence).collect();
        assert_eq!(occurrences, vec![10, 11, 20]);
    }

    #[test]
    fn drain_due_is_deterministic_across_runs() {
        let gen1 = generation(1);
        let due = SemanticTimeMicros::from_micros(5_000);
        let build = || {
            let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
            for occurrence in [3_u64, 1, 2] {
                lane.schedule(
                    gen1,
                    ordinal(1),
                    TestFamily::AiThink,
                    occurrence,
                    None,
                    due,
                    10,
                )
                .expect("schedule");
            }
            lane
        };

        let clock = VirtualOwnerClock::new(due);
        let mut first = build();
        let mut second = build();
        let fired_first = first.drain_due(&clock, gen1, |_| true);
        let fired_second = second.drain_due(&clock, gen1, |_| true);
        let ids_first: Vec<u64> = fired_first.iter().map(|timer| timer.occurrence).collect();
        let ids_second: Vec<u64> = fired_second.iter().map(|timer| timer.occurrence).collect();
        assert_eq!(ids_first, ids_second);
        // Same due time and ordinal for all three: order is call-order sequence, not value.
        assert_eq!(ids_first, vec![3, 1, 2]);
    }

    #[test]
    fn not_yet_due_timer_does_not_fire() {
        let gen1 = generation(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
        lane.schedule(
            gen1,
            ordinal(1),
            TestFamily::AiThink,
            1,
            None,
            SemanticTimeMicros::from_micros(2_000),
            10,
        )
        .expect("schedule");

        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(1_999));
        assert!(lane.drain_due(&clock, gen1, |_| true).is_empty());
        assert_eq!(lane.pending_len(), 1);

        clock.advance(1);
        let fired = lane.drain_due(&clock, gen1, |_| true);
        assert_eq!(fired.len(), 1);
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn fired_timer_never_fires_again() {
        let gen1 = generation(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
        let due = SemanticTimeMicros::from_micros(100);
        lane.schedule(gen1, ordinal(1), TestFamily::AiThink, 1, None, due, 10)
            .expect("schedule");

        let clock = VirtualOwnerClock::new(due);
        let first = lane.drain_due(&clock, gen1, |_| true);
        assert_eq!(first.len(), 1);
        // No busy loop: a second drain at the same or later time returns nothing for an
        // already-fired occurrence, and scheduling a fresh occurrence after the drain fires
        // only on its own later call (FND-03 §10.4: no zero-delay recursion within one drain).
        let second = lane.drain_due(&clock, gen1, |_| true);
        assert!(second.is_empty());

        lane.schedule(gen1, ordinal(2), TestFamily::AiThink, 2, None, due, 10)
            .expect("reschedule new occurrence");
        let third = lane.drain_due(&clock, gen1, |_| true);
        assert_eq!(third.len(), 1);
        assert_eq!(third[0].occurrence, 2);
    }

    #[test]
    fn pending_limit_max_accepted_max_plus_one_rejected() {
        // AI01-PENDING-TIMERS-PER-ACTOR: at most AI01_PENDING_TIMERS_PER_ACTOR pending think
        // timers per creature actor; the second schedule attempt is rejected.
        let gen1 = generation(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
        let creature = actor(10);
        let due = SemanticTimeMicros::from_micros(1_000);

        lane.schedule(
            gen1,
            ordinal(1),
            TestFamily::AiThink,
            1,
            Some(creature),
            due,
            AI01_PENDING_TIMERS_PER_ACTOR,
        )
        .expect("max accepted");
        assert_eq!(lane.pending_for_key(TestFamily::AiThink, Some(creature)), 1);

        let rejected = lane.schedule(
            gen1,
            ordinal(2),
            TestFamily::AiThink,
            2,
            Some(creature),
            due,
            AI01_PENDING_TIMERS_PER_ACTOR,
        );
        assert_eq!(rejected, Err(OwnerTimerError::PendingLimitReached));
        assert_eq!(lane.pending_for_key(TestFamily::AiThink, Some(creature)), 1);

        // A different creature is a different key and is unaffected.
        let other = actor(11);
        lane.schedule(
            gen1,
            ordinal(3),
            TestFamily::AiThink,
            3,
            Some(other),
            due,
            AI01_PENDING_TIMERS_PER_ACTOR,
        )
        .expect("different key accepted");
    }

    #[test]
    fn duplicate_occurrence_identity_never_scheduled_twice() {
        let gen1 = generation(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
        let due = SemanticTimeMicros::from_micros(1_000);
        lane.schedule(gen1, ordinal(1), TestFamily::Respawn, 7, None, due, 4)
            .expect("first schedule");
        let rejected = lane.schedule(gen1, ordinal(2), TestFamily::Respawn, 7, None, due, 4);
        assert_eq!(rejected, Err(OwnerTimerError::DuplicateOccurrence));
        assert_eq!(lane.pending_len(), 1);
    }

    #[test]
    fn cancel_removes_pending_occurrence() {
        let gen1 = generation(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
        let due = SemanticTimeMicros::from_micros(1_000);
        lane.schedule(gen1, ordinal(1), TestFamily::Respawn, 1, None, due, 4)
            .expect("schedule");
        assert!(lane.cancel(TestFamily::Respawn, 1));
        assert!(!lane.cancel(TestFamily::Respawn, 1));
        assert_eq!(lane.pending_len(), 0);

        let clock = VirtualOwnerClock::new(due);
        assert!(lane.drain_due(&clock, gen1, |_| true).is_empty());
    }

    #[test]
    fn schedule_rejects_stale_owner_generation() {
        let gen1 = generation(1);
        let gen2 = generation(2);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
        let rejected = lane.schedule(
            gen2,
            ordinal(1),
            TestFamily::AiThink,
            1,
            None,
            SemanticTimeMicros::from_micros(1),
            10,
        );
        assert_eq!(rejected, Err(OwnerTimerError::StaleOwnerGeneration));
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn drain_due_fires_nothing_for_stale_lane_generation() {
        // A timer scheduled under the lane's own generation never fires once the caller has
        // moved to a new ownership generation and drains with that new value: a superseded
        // owner cannot pull mutation-shaped input out of a lane it no longer owns.
        let gen1 = generation(1);
        let gen2 = generation(2);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
        let due = SemanticTimeMicros::from_micros(10);
        lane.schedule(gen1, ordinal(1), TestFamily::AiThink, 1, None, due, 10)
            .expect("schedule under gen1");

        let clock = VirtualOwnerClock::new(due);
        assert!(lane.drain_due(&clock, gen2, |_| true).is_empty());
        // The entry is untouched: the caller must build a fresh lane for gen2, not keep
        // draining this one.
        assert_eq!(lane.pending_len(), 1);
    }

    #[test]
    fn drain_due_drops_stale_target_without_returning_it() {
        // A timer from a stale target (actor-local) generation never fires: `target_is_current`
        // stands in for the owner's live-state check (FND-03 §10.3), since `ExactActorRef`
        // equality already binds identity and generation together and this generic lane cannot
        // (and must not) decode a foreign actor handle itself.
        let gen1 = generation(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
        let stale_actor = actor(12);
        let due = SemanticTimeMicros::from_micros(500);
        lane.schedule(
            gen1,
            ordinal(1),
            TestFamily::AiThink,
            1,
            Some(stale_actor),
            due,
            AI01_PENDING_TIMERS_PER_ACTOR,
        )
        .expect("schedule");

        let clock = VirtualOwnerClock::new(due);
        // No live actor currently matches `stale_actor` (it respawned under a new generation).
        let fired = lane.drain_due(&clock, gen1, |_| false);
        assert!(fired.is_empty());
        // The stale entry is consumed, not left pending forever.
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn drain_due_keeps_current_target_and_ignores_others() {
        let gen1 = generation(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(gen1);
        let live_actor = actor(13);
        let dead_actor = actor(14);
        let due = SemanticTimeMicros::from_micros(500);
        lane.schedule(
            gen1,
            ordinal(1),
            TestFamily::AiThink,
            1,
            Some(live_actor),
            due,
            AI01_PENDING_TIMERS_PER_ACTOR,
        )
        .expect("schedule live");
        lane.schedule(
            gen1,
            ordinal(2),
            TestFamily::Respawn,
            2,
            Some(dead_actor),
            due,
            4,
        )
        .expect("schedule dead");

        let clock = VirtualOwnerClock::new(due);
        let fired = lane.drain_due(&clock, gen1, |target| target == live_actor);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].occurrence, 1);
    }
}
