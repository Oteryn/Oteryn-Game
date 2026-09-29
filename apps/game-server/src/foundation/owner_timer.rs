//! Channel owner timer lane (FND-03 §10), first implemented for
//! GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1 §4.2
//! (`docs/architecture/reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md`).
//!
//! FND-03 §8.4 selects no universal fixed tick: an authoritative scope executes bounded
//! ordered work cycles, and think work, respawn work and (later) spell cooldowns/regeneration
//! all arrive as owner-scoped timer inputs, never as direct callbacks (FND-03 §10). One
//! `OwnerTimerLane` instance belongs to exactly one Channel owner for exactly one exact scope
//! (`RuntimeScopeRefV1`) at exactly one `ScopeOwnershipGeneration`; a scope move creates a new
//! lane under the new generation.
//!
//! The lane is deliberately generic over the caller's `Family` (AI think, respawn, and later
//! spell cooldown/regeneration) and `Occurrence` identity, so this stays one shared mechanism
//! rather than one bespoke scheduler per gameplay system (§4.2: "serves AI now and spell
//! cooldowns and regeneration later").
//!
//! What this module does NOT do: it never mutates game state itself. `drain_due` only returns
//! the timers whose deadline has passed, in FND-03 §10.1 deterministic order, for the owner to
//! apply as normalized inputs under its own `ScopeRuntimeFence`/`RuntimeWorkStamp`. A
//! fired timer whose target no longer matches the owner's current state is dropped without
//! mutation (FND-03 §10.3) and never returned. A pending entry whose target has gone stale is
//! purged during drain even before its own deadline, so death/respawn churn cannot accumulate
//! stale entries (FND-03 §10.3).
//!
//! `schedule` and `drain_due` both require the caller's `&ScopeRuntimeFence`: the single
//! mutated-in-place owner-cycle authority for this lane's exact scope and ownership generation
//! (the same fence `RuntimeWorkStamp` issuance already uses). Comparing only two
//! construction-time-fixed values (a caller-supplied generation against the lane's own stored
//! field) can never detect a real handoff, since neither value changes after the fact; the
//! fence does, because the authoritative handoff mutates it in place. A superseded owner still
//! holding this lane and a reference to that same fence therefore observes the move and is
//! refused rather than draining or scheduling mutation-shaped input under a generation it no
//! longer holds. The fence must also be bound to this lane's *exact scope*
//! (`ScopeRuntimeFence::is_current_for_scope`): a raw generation number alone cannot distinguish
//! Channel A from Channel B when both happen to reach the same generation, so a fence granted
//! for Channel B never authorizes Channel A's lane even at a matching generation number
//! (FND-03 §10.3).
//!
//! `schedule`'s `scheduling_stamp: RuntimeWorkStamp` must itself have been issued by that same
//! fence (`ScopeRuntimeFence::accept_input` + `stamp`) for the lane's generation: a raw,
//! caller-fabricated ordinal is never accepted, only one the fence actually vouches for as
//! current (FND-03 §10.1/§10.3).
//!
//! Each family's pending-per-key cap and catch-up policy (`FamilyPolicy`) are fixed once at
//! construction, validated against a registered hard maximum the caller supplies alongside it,
//! and never taken as a per-`schedule`-call argument (§4.9: `AI01_PENDING_TIMERS_PER_ACTOR` for
//! the AI think family). The catch-up policy (`CatchUpPolicy::SkipToLatest` at minimum, for
//! think timers per §4.2 / FND-03 §10.5) is enforced by `drain_due` itself: it never returns
//! more than one fired occurrence for one (family, target) key in a single call, and a
//! `SkipToLatest` family reports the clock's current time as the fired timer's `due`, never the
//! stale original deadline, so a caller that reschedules relative to `due` cannot be driven to
//! replay every missed interval one at a time.

use super::{
    ExactActorRef, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeOwnershipGeneration, ScopeRuntimeFence,
};

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
    /// The caller's `&ScopeRuntimeFence` no longer proves this lane's exact scope and
    /// ownership generation are current (its scope or generation differs, the fence was
    /// invalidated, or a `schedule` call's `RuntimeWorkStamp` was not issued by this same fence
    /// for this generation). The lane never re-binds to a new scope or generation; a scope move
    /// or handoff must build a fresh lane.
    StaleOwnerGeneration,
    /// The same (family, occurrence) identity already has a pending entry (FND-03 §10.1: "the
    /// same identity is never scheduled twice").
    DuplicateOccurrence,
    /// The per-family pending cap fixed at lane construction for this (family, target) key is
    /// already reached (for example AI01-PENDING-TIMERS-PER-ACTOR: at most one pending think
    /// timer per creature). No caller argument can raise this above the constructed cap.
    PendingLimitReached,
    /// A `family_policies` entry passed to `for_generation` requested a pending cap above the
    /// registered hard maximum of that family (`TimerFamily::registered_maximum`) (AI-RL-06 /
    /// `AI01_PENDING_TIMERS_PER_ACTOR`): construction is refused rather than silently clamped.
    FamilyCapExceedsRegisteredMaximum,
}

impl std::fmt::Display for OwnerTimerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::StaleOwnerGeneration => "timer schedule named a stale owner authority",
            Self::DuplicateOccurrence => "timer occurrence identity already scheduled",
            Self::PendingLimitReached => "timer pending-per-key limit reached",
            Self::FamilyCapExceedsRegisteredMaximum => {
                "family pending cap exceeds its registered hard maximum"
            }
        };
        formatter.write_str(text)
    }
}

impl std::error::Error for OwnerTimerError {}

/// AI-RL-06 / `AI01-PENDING-TIMERS-PER-ACTOR` (`RESOURCE_LIMITS_REGISTRY.json`): at most one
/// pending AI think timer per live creature actor. Registered by AI-1 (§4.9 of the decision);
/// the AI think family's `TimerFamily::registered_maximum` returns this value.
pub const AI01_PENDING_TIMERS_PER_ACTOR: usize = 1;

/// A timer family's registered hard maximum of pending timers per (family, target) key, taken
/// from `RESOURCE_LIMITS_REGISTRY.json` by the family type itself, never from a caller: a lane's
/// constructed per-family cap is checked against it, so no constructor argument can exceed the
/// registered row (AI-RL-06: `AI01_PENDING_TIMERS_PER_ACTOR` for AI think timers).
pub trait TimerFamily: Copy + Eq {
    fn registered_maximum(self) -> usize;
}

/// A family's fixed catch-up behavior for a badly overdue timer (FND-03 §10.5 taxonomy; §4.9
/// assigns `SKIP_TO_LATEST` to AI think and `DEADLINE_STATE` to respawn).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatchUpPolicy {
    /// A target that missed one or more intervals fires exactly once per `drain_due` call for
    /// its (family, target) key: earlier missed occurrences for the same key are dropped
    /// without being returned (never mutated, matching the stale-target purge FND-03 §10.3
    /// already applies), and the fired timer reports `clock.now()` as its `due` rather than the
    /// stale original deadline, so a caller that reschedules relative to `due` (for example
    /// `due + interval`) computes the next occurrence from the current clock instead of
    /// replaying every missed interval one at a time. Minimum policy for think timers
    /// (§4.2 / §4.9 / FND-03 §10.5).
    SkipToLatest,
    /// Every due entry fires as scheduled, reporting its own original `due`; entries are never
    /// collapsed by key. Distinct pending entries at the same key are each independent, durable
    /// deadline state (§4.9's respawn policy: "at most the spawn's population pending" — each
    /// dead creature's own respawn timer must still be delivered, not collapsed into one).
    DeadlineState,
}

/// One family's fixed pending-per-key cap and catch-up policy (§4.9 / FND-03 §10.5), set once
/// at lane construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyPolicy {
    pub max_pending: usize,
    pub catch_up: CatchUpPolicy,
}

#[derive(Debug, Clone, Copy)]
struct ScheduledTimer<Family, Occurrence> {
    family: Family,
    occurrence: Occurrence,
    target: Option<ExactActorRef>,
    due: SemanticTimeMicros,
    scheduling_stamp: RuntimeWorkStamp,
    sequence: u64,
}

/// A timer whose deadline has passed, returned by `drain_due` for the caller to apply as one
/// normalized owner input (FND-03 §10.2). `due` is the fired occurrence's effective deadline:
/// for a `CatchUpPolicy::SkipToLatest` family it is `clock.now()`, never the (possibly long
/// overdue) originally scheduled deadline.
#[derive(Debug, Clone, Copy)]
pub struct FiredTimer<Family, Occurrence> {
    pub family: Family,
    pub occurrence: Occurrence,
    pub target: Option<ExactActorRef>,
    pub due: SemanticTimeMicros,
}

/// One Channel owner's timer lane for one exact scope and `ScopeOwnershipGeneration`
/// (FND-03 §10).
#[derive(Debug)]
pub struct OwnerTimerLane<Family, Occurrence> {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    entries: Vec<ScheduledTimer<Family, Occurrence>>,
    next_sequence: u64,
    /// Per-family policy, fixed once at construction (§4.9 / `AI01_PENDING_TIMERS_PER_ACTOR`).
    /// `schedule` takes no per-call cap argument, so no caller can raise a family's cap above
    /// what the lane was built with. A family with no entry here has cap 0 (fail closed): it
    /// can never hold a pending timer.
    family_policies: Vec<(Family, FamilyPolicy)>,
}

impl<Family, Occurrence> OwnerTimerLane<Family, Occurrence>
where
    Family: TimerFamily,
    Occurrence: Copy + Eq,
{
    /// `scope` fixes the exact Channel/Instance identity this lane's authority proofs must be
    /// bound to (owner_timer.rs P1): a fence proving `generation` for a *different* scope never
    /// authorizes this lane, even if the numeric generation matches.
    ///
    /// `family_policies` fixes, once, every `Family` variant this lane will accept, its
    /// per-(family, target) pending cap and its catch-up policy (for the AI think family, the
    /// registered `AI01_PENDING_TIMERS_PER_ACTOR` and `CatchUpPolicy::SkipToLatest`). This is a
    /// policy decision for the lane's owner to make at construction, never a per-`schedule`-call
    /// argument. Each family's `TimerFamily::registered_maximum` is the hard ceiling its
    /// `max_pending` is checked against; a requested cap above it is refused
    /// (`FamilyCapExceedsRegisteredMaximum`) rather than silently accepted or clamped.
    pub fn for_generation(
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        family_policies: impl IntoIterator<Item = (Family, FamilyPolicy)>,
    ) -> Result<Self, OwnerTimerError> {
        let mut resolved: Vec<(Family, FamilyPolicy)> = Vec::new();
        for (family, policy) in family_policies {
            if policy.max_pending > family.registered_maximum() {
                return Err(OwnerTimerError::FamilyCapExceedsRegisteredMaximum);
            }
            resolved.push((family, policy));
        }
        Ok(Self {
            scope,
            generation,
            entries: Vec::new(),
            next_sequence: 0,
            family_policies: resolved,
        })
    }

    #[must_use]
    pub const fn scope(&self) -> RuntimeScopeRefV1 {
        self.scope
    }

    #[must_use]
    pub const fn generation(&self) -> ScopeOwnershipGeneration {
        self.generation
    }

    /// The fixed policy for `family` (cap 0, `SkipToLatest`, fail closed, if `family` was not
    /// given a policy at construction).
    #[must_use]
    fn policy_for_family(&self, family: Family) -> FamilyPolicy {
        self.family_policies
            .iter()
            .find(|(candidate, _)| *candidate == family)
            .map_or(
                FamilyPolicy {
                    max_pending: 0,
                    catch_up: CatchUpPolicy::SkipToLatest,
                },
                |(_, policy)| *policy,
            )
    }

    /// Number of pending timers currently occupying this exact `(family, target)` key, the
    /// same key `schedule`'s registered cap bounds.
    #[must_use]
    pub fn pending_for_key(&self, family: Family, target: Option<ExactActorRef>) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.family == family && entry.target == target)
            .count()
    }

    /// Schedules one owner-scoped timer (FND-03 §10.1).
    ///
    /// `scheduling_stamp` must be a `RuntimeWorkStamp` this same `owner_fence` issued
    /// (`ScopeRuntimeFence::accept_input` + `stamp`) for the current resolution: it is rejected
    /// whenever the fence no longer vouches for it (`ScopeRuntimeFence::accepts_stamp`), so a
    /// raw, caller-fabricated ordinal can never stand in for one the fence actually granted.
    /// Ties at an equal `due` order by that stamp's ordinal, then by an internal monotonically
    /// increasing sequence assigned in call order (FND-03 §10.1: "a deterministic
    /// within-resolution sequence; a separate globally visible timer counter is not required").
    ///
    /// The pending-per-key cap for `family` is the value fixed for it at lane construction (for
    /// example `AI01_PENDING_TIMERS_PER_ACTOR`), never a per-call argument.
    ///
    /// `owner_fence` must independently prove this lane's exact scope and `generation` are
    /// still the *current* owner authority (FND-04 scope assignment continuity): it is rejected
    /// whenever `owner_fence`'s own scope or generation differs from this lane's, or the fence
    /// was invalidated, even if some other caller-held value would still equal this lane's
    /// stored generation.
    pub fn schedule(
        &mut self,
        owner_fence: &ScopeRuntimeFence,
        scheduling_stamp: RuntimeWorkStamp,
        family: Family,
        occurrence: Occurrence,
        target: Option<ExactActorRef>,
        due: SemanticTimeMicros,
    ) -> Result<(), OwnerTimerError> {
        if !owner_fence.is_current_for_scope(self.scope, self.generation)
            || !owner_fence.accepts_stamp(scheduling_stamp)
        {
            return Err(OwnerTimerError::StaleOwnerGeneration);
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.family == family && entry.occurrence == occurrence)
        {
            return Err(OwnerTimerError::DuplicateOccurrence);
        }
        if self.pending_for_key(family, target) >= self.policy_for_family(family).max_pending {
            return Err(OwnerTimerError::PendingLimitReached);
        }
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.entries.push(ScheduledTimer {
            family,
            occurrence,
            target,
            due,
            scheduling_stamp,
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
    /// `owner_fence` must independently prove this lane's exact scope and `generation` are
    /// still the *current* owner authority, exactly as `schedule` requires; a lane whose scope
    /// or generation `owner_fence` no longer confirms as current fires nothing (FND-03 §10.3)
    /// and mutates nothing, even when a caller-held raw generation value would still equal this
    /// lane's stored field. The caller must build a fresh lane for a new scope/generation
    /// rather than keep draining a superseded one.
    ///
    /// For every entry with a `target`, `target_is_current` decides whether that target's
    /// generation still matches the owner's live state, checked whether or not the entry is
    /// already due: a `false` purges the entry without mutation and without returning it
    /// (matching FND-03 §10.3's target-generation cancellation), so death/respawn churn on a
    /// target cannot accumulate a stale not-yet-due entry either. This lane never mutates game
    /// state itself: it only classifies and returns normalized inputs for the owner to apply.
    ///
    /// Catch-up (FND-03 §10.5): once due entries are collected, a family whose `CatchUpPolicy`
    /// is `SkipToLatest` collapses every (family, target) key to its single latest due
    /// occurrence — earlier missed occurrences for that key are dropped without being returned
    /// — and the surviving fired timer reports `clock.now()` as its `due`, not the stale
    /// original deadline.
    pub fn drain_due(
        &mut self,
        clock: &impl OwnerClock,
        owner_fence: &ScopeRuntimeFence,
        mut target_is_current: impl FnMut(ExactActorRef) -> bool,
    ) -> Vec<FiredTimer<Family, Occurrence>> {
        if !owner_fence.is_current_for_scope(self.scope, self.generation) {
            return Vec::new();
        }
        let now = clock.now();
        let mut due: Vec<ScheduledTimer<Family, Occurrence>> = Vec::new();
        self.entries.retain(|entry| {
            if let Some(target) = entry.target
                && !target_is_current(target)
            {
                // Stale target: purge now even if not yet due (FND-03 §10.3).
                return false;
            }
            if entry.due <= now {
                due.push(*entry);
                false
            } else {
                true
            }
        });
        due.sort_by(|left, right| {
            (left.due, left.scheduling_stamp.ordinal(), left.sequence).cmp(&(
                right.due,
                right.scheduling_stamp.ordinal(),
                right.sequence,
            ))
        });

        // FND-03 §10.5 catch-up: a `SkipToLatest` family collapses a key that missed several
        // intervals down to its single latest due occurrence. `due` is sorted ascending, so
        // later entries for the same key always replace earlier ones here; the earlier missed
        // occurrences are dropped without ever being returned or mutated.
        let mut kept: Vec<ScheduledTimer<Family, Occurrence>> = Vec::new();
        for entry in due {
            let collapses =
                self.policy_for_family(entry.family).catch_up == CatchUpPolicy::SkipToLatest;
            if collapses
                && let Some(existing) = kept.iter_mut().find(|kept_entry| {
                    kept_entry.family == entry.family && kept_entry.target == entry.target
                })
            {
                *existing = entry;
                continue;
            }
            kept.push(entry);
        }
        // A replacement keeps its key's earlier slot, so restore deadline order afterwards.
        kept.sort_by(|left, right| {
            (left.due, left.scheduling_stamp.ordinal(), left.sequence).cmp(&(
                right.due,
                right.scheduling_stamp.ordinal(),
                right.sequence,
            ))
        });

        kept.into_iter()
            .map(|entry| {
                let report_now =
                    self.policy_for_family(entry.family).catch_up == CatchUpPolicy::SkipToLatest;
                FiredTimer {
                    family: entry.family,
                    occurrence: entry.occurrence,
                    target: entry.target,
                    due: if report_now { now } else { entry.due },
                }
            })
            .collect()
    }
}

/// `COMBAT01-CORPSES-PER-SCOPE` (VSL resource-rows decision §4.1 row 6, reused unchanged by the
/// D3 decision §4.2): at most 64 live corpses per scope. It is the registered hard maximum of the
/// corpse decay family below: one pending decay timer per live corpse, all sharing the family's
/// single target-less key, so no new resource row is needed.
pub const COMBAT01_CORPSES_PER_SCOPE: usize = 64;

/// D135 (`D3-CORPSE-CONTAINER-LOOT-WINDOW-DECAY-V1` §4.6, child D3-6): a corpse decays at the
/// durable absolute deadline `materialized_at + 60 s`.
pub const CORPSE_DECAY_AFTER_MS: i64 = 60_000;

/// D135: the corpse decay timer family. Its timers carry no actor target (a corpse is an
/// `ItemInstance`, not an actor) and fire as durable deadline state: every scheduled corpse's own
/// decay is delivered, never collapsed with another's (`CatchUpPolicy::DeadlineState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpseDecayFamily;

impl TimerFamily for CorpseDecayFamily {
    fn registered_maximum(self) -> usize {
        COMBAT01_CORPSES_PER_SCOPE
    }
}

/// The fixed decay family policy a lane is constructed with.
pub const CORPSE_DECAY_POLICY: FamilyPolicy = FamilyPolicy {
    max_pending: COMBAT01_CORPSES_PER_SCOPE,
    catch_up: CatchUpPolicy::DeadlineState,
};

/// One corpse's decay occurrence: the corpse `ItemInstanceId`, which decays at most once, so it
/// is never scheduled twice in one lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpseDecayOccurrence {
    pub corpse_item_instance_id: [u8; 16],
}

/// Map a corpse's durable decay deadline (database-clock unix ms) onto this owner's monotonic
/// timer clock: `now` plus the time still remaining at `database_now_unix_ms`, both read together
/// (the recovery query returns them in one statement). An already-passed deadline is due now, so
/// a corpse whose decay was missed or interrupted (a restart, a handoff, a partly drained corpse)
/// fires on the next drain. The timer is never set later than the deadline; the durable commit
/// independently refuses a retirement before it, so decay is never early either.
#[must_use]
pub fn corpse_decay_due(
    now: SemanticTimeMicros,
    database_now_unix_ms: i64,
    decay_at_unix_ms: i64,
) -> SemanticTimeMicros {
    let remaining_ms = decay_at_unix_ms.saturating_sub(database_now_unix_ms).max(0);
    now.saturating_add_micros(
        u64::try_from(remaining_ms)
            .unwrap_or(0)
            .saturating_mul(1_000),
    )
}

/// Schedule one corpse's decay: at corpse-MINT commit and again, from the durable recovery query,
/// at every scope (re)admission into the new owner's fresh lane. It inherits every
/// `OwnerTimerLane::schedule` check (fence, stamp, duplicate identity, the registered cap).
pub fn schedule_corpse_decay(
    lane: &mut OwnerTimerLane<CorpseDecayFamily, CorpseDecayOccurrence>,
    owner_fence: &ScopeRuntimeFence,
    scheduling_stamp: RuntimeWorkStamp,
    corpse_item_instance_id: [u8; 16],
    due: SemanticTimeMicros,
) -> Result<CorpseDecayOccurrence, OwnerTimerError> {
    let occurrence = CorpseDecayOccurrence {
        corpse_item_instance_id,
    };
    lane.schedule(
        owner_fence,
        scheduling_stamp,
        CorpseDecayFamily,
        occurrence,
        None,
        due,
    )?;
    Ok(occurrence)
}

#[cfg(test)]
#[allow(clippy::expect_used)] // Test-only: fixture construction and schedule() calls whose
// success is the test's own precondition, not the behavior under test.
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, WorldId};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestFamily {
        AiThink,
        Respawn,
    }

    impl TimerFamily for TestFamily {
        fn registered_maximum(self) -> usize {
            match self {
                Self::AiThink => AI01_PENDING_TIMERS_PER_ACTOR,
                // No registered row in this slice; a generous test-only ceiling.
                Self::Respawn => 16,
            }
        }
    }

    #[allow(clippy::expect_used)]
    fn generation(value: u64) -> ScopeOwnershipGeneration {
        ScopeOwnershipGeneration::new(value).expect("nonzero generation")
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
    fn world_id() -> WorldId {
        WorldId::decode(&uuid_v7(1)).expect("valid UUIDv7 WorldId")
    }

    #[allow(clippy::expect_used)]
    fn channel_id(channel_seed: u8) -> ChannelId {
        ChannelId::decode(&uuid_v7(channel_seed)).expect("valid UUIDv7 ChannelId")
    }

    #[allow(clippy::expect_used)]
    fn actor(channel_seed: u8) -> ExactActorRef {
        ExactActorRef::transport_fixture(world_id(), channel_id(channel_seed))
    }

    /// The exact Channel scope identity a lane and its fence are bound to in a test.
    fn scope_for(channel_seed: u8) -> RuntimeScopeRefV1 {
        RuntimeScopeRefV1::channel(world_id(), channel_id(channel_seed))
    }

    /// A fresh, independent `ScopeRuntimeFence` granted for `generation` and bound to `scope`:
    /// the live owner-cycle authority `schedule`/`drain_due` require proof from.
    fn fence(scope: RuntimeScopeRefV1, generation: ScopeOwnershipGeneration) -> ScopeRuntimeFence {
        ScopeRuntimeFence::from_external_grant(generation).with_scope(scope)
    }

    /// A `RuntimeWorkStamp` `owner_fence` itself issues for `generation`, exactly as a real
    /// owner resolution obtains one from `accept_input` before calling `schedule` (module doc:
    /// "the same fence `RuntimeWorkStamp` issuance already uses").
    #[allow(clippy::expect_used)]
    fn issue_stamp(
        owner_fence: &mut ScopeRuntimeFence,
        generation: ScopeOwnershipGeneration,
    ) -> RuntimeWorkStamp {
        let ordinal = owner_fence
            .accept_input(generation)
            .expect("issue a live ordinal");
        owner_fence.stamp(ordinal)
    }

    /// The default test lane: generous per-family caps that never themselves reject a
    /// `schedule` call for tests not exercising the pending-limit or cap-ceiling policy. The AI
    /// think family keeps the real registered ceiling (`AI01_PENDING_TIMERS_PER_ACTOR`); Respawn
    /// gets a generous one since it has no registered ceiling in this slice.
    fn generous_lane(
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> OwnerTimerLane<TestFamily, u64> {
        OwnerTimerLane::for_generation(
            scope,
            generation,
            [
                (
                    TestFamily::AiThink,
                    FamilyPolicy {
                        max_pending: AI01_PENDING_TIMERS_PER_ACTOR,
                        catch_up: CatchUpPolicy::SkipToLatest,
                    },
                ),
                (
                    TestFamily::Respawn,
                    FamilyPolicy {
                        max_pending: 10,
                        catch_up: CatchUpPolicy::DeadlineState,
                    },
                ),
            ],
        )
        .expect("caps within their registered maximum")
    }

    #[test]
    fn equal_deadline_orders_by_scheduling_ordinal_then_sequence() {
        // Respawn (not AiThink): this test exercises tie-break ordering, not the pending-cap
        // policy, so it needs more than one concurrent pending entry at the same (family,
        // target) key, which the tightly-capped AiThink family cannot hold.
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let due = SemanticTimeMicros::from_micros(1_000);

        let stamp_1 = issue_stamp(&mut owner_fence, gen1);
        let stamp_2 = issue_stamp(&mut owner_fence, gen1);

        // Later ordinal scheduled first in call order, but must still fire after the earlier
        // ordinal at the same deadline.
        lane.schedule(&owner_fence, stamp_2, TestFamily::Respawn, 20, None, due)
            .expect("schedule ordinal 2");
        lane.schedule(&owner_fence, stamp_1, TestFamily::Respawn, 10, None, due)
            .expect("schedule ordinal 1");
        // Same ordinal as the first entry: must fire after it, by call-order sequence.
        lane.schedule(&owner_fence, stamp_1, TestFamily::Respawn, 11, None, due)
            .expect("schedule ordinal 1 second");

        let clock = VirtualOwnerClock::new(due);
        let fired = lane.drain_due(&clock, &owner_fence, |_| true);
        let occurrences: Vec<u64> = fired.iter().map(|timer| timer.occurrence).collect();
        assert_eq!(occurrences, vec![10, 11, 20]);
    }

    #[test]
    fn drain_due_is_deterministic_across_runs() {
        let gen1 = generation(1);
        let scope = scope_for(1);
        let due = SemanticTimeMicros::from_micros(5_000);
        let mut owner_fence = fence(scope, gen1);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        let build = |owner_fence: &ScopeRuntimeFence| {
            let mut lane = generous_lane(scope, gen1);
            for occurrence in [3_u64, 1, 2] {
                lane.schedule(
                    owner_fence,
                    stamp,
                    TestFamily::Respawn,
                    occurrence,
                    None,
                    due,
                )
                .expect("schedule");
            }
            lane
        };

        let clock = VirtualOwnerClock::new(due);
        let mut first = build(&owner_fence);
        let mut second = build(&owner_fence);
        let fired_first = first.drain_due(&clock, &owner_fence, |_| true);
        let fired_second = second.drain_due(&clock, &owner_fence, |_| true);
        let ids_first: Vec<u64> = fired_first.iter().map(|timer| timer.occurrence).collect();
        let ids_second: Vec<u64> = fired_second.iter().map(|timer| timer.occurrence).collect();
        assert_eq!(ids_first, ids_second);
        // Same due time and ordinal for all three: order is call-order sequence, not value.
        assert_eq!(ids_first, vec![3, 1, 2]);
    }

    #[test]
    fn not_yet_due_timer_does_not_fire() {
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp,
            TestFamily::AiThink,
            1,
            None,
            SemanticTimeMicros::from_micros(2_000),
        )
        .expect("schedule");

        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(1_999));
        assert!(lane.drain_due(&clock, &owner_fence, |_| true).is_empty());
        assert_eq!(lane.pending_len(), 1);

        clock.advance(1);
        let fired = lane.drain_due(&clock, &owner_fence, |_| true);
        assert_eq!(fired.len(), 1);
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn fired_timer_never_fires_again() {
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let due = SemanticTimeMicros::from_micros(100);
        let stamp_1 = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(&owner_fence, stamp_1, TestFamily::AiThink, 1, None, due)
            .expect("schedule");

        let clock = VirtualOwnerClock::new(due);
        let first = lane.drain_due(&clock, &owner_fence, |_| true);
        assert_eq!(first.len(), 1);
        // No busy loop: a second drain at the same or later time returns nothing for an
        // already-fired occurrence, and scheduling a fresh occurrence after the drain fires
        // only on its own later call (FND-03 §10.4: no zero-delay recursion within one drain).
        let second = lane.drain_due(&clock, &owner_fence, |_| true);
        assert!(second.is_empty());

        let stamp_2 = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(&owner_fence, stamp_2, TestFamily::AiThink, 2, None, due)
            .expect("reschedule new occurrence");
        let third = lane.drain_due(&clock, &owner_fence, |_| true);
        assert_eq!(third.len(), 1);
        assert_eq!(third[0].occurrence, 2);
    }

    #[test]
    fn pending_limit_max_accepted_max_plus_one_rejected() {
        // AI01-PENDING-TIMERS-PER-ACTOR: at most AI01_PENDING_TIMERS_PER_ACTOR pending think
        // timers per creature actor; the second schedule attempt is rejected. The cap is fixed
        // at lane construction, not passed by the caller of `schedule`.
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = OwnerTimerLane::for_generation(
            scope,
            gen1,
            [(
                TestFamily::AiThink,
                FamilyPolicy {
                    max_pending: AI01_PENDING_TIMERS_PER_ACTOR,
                    catch_up: CatchUpPolicy::SkipToLatest,
                },
            )],
        )
        .expect("cap within registered maximum");
        let mut owner_fence = fence(scope, gen1);
        let creature = actor(10);
        let due = SemanticTimeMicros::from_micros(1_000);

        let stamp_1 = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp_1,
            TestFamily::AiThink,
            1,
            Some(creature),
            due,
        )
        .expect("max accepted");
        assert_eq!(lane.pending_for_key(TestFamily::AiThink, Some(creature)), 1);

        let stamp_2 = issue_stamp(&mut owner_fence, gen1);
        let rejected = lane.schedule(
            &owner_fence,
            stamp_2,
            TestFamily::AiThink,
            2,
            Some(creature),
            due,
        );
        assert_eq!(rejected, Err(OwnerTimerError::PendingLimitReached));
        assert_eq!(lane.pending_for_key(TestFamily::AiThink, Some(creature)), 1);

        // A different creature is a different key and is unaffected.
        let other = actor(11);
        let stamp_3 = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp_3,
            TestFamily::AiThink,
            3,
            Some(other),
            due,
        )
        .expect("different key accepted");
    }

    #[test]
    fn family_cap_is_fixed_at_construction_and_cannot_be_raised_per_call() {
        // The P2 concern: `schedule` no longer takes a per-call cap argument at all, so there is
        // no value any caller could pass to raise a family's cap above what the lane was built
        // with (here, the registered AI01_PENDING_TIMERS_PER_ACTOR = 1). A second schedule for
        // the same (family, target) key is rejected regardless of what the caller might have
        // wanted, since there is no longer any parameter through which to ask for more.
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = OwnerTimerLane::for_generation(
            scope,
            gen1,
            [(
                TestFamily::AiThink,
                FamilyPolicy {
                    max_pending: AI01_PENDING_TIMERS_PER_ACTOR,
                    catch_up: CatchUpPolicy::SkipToLatest,
                },
            )],
        )
        .expect("cap within registered maximum");
        let mut owner_fence = fence(scope, gen1);
        let creature = actor(30);
        let due = SemanticTimeMicros::from_micros(1_000);

        let stamp_1 = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp_1,
            TestFamily::AiThink,
            1,
            Some(creature),
            due,
        )
        .expect("first think timer accepted");
        let stamp_2 = issue_stamp(&mut owner_fence, gen1);
        let rejected = lane.schedule(
            &owner_fence,
            stamp_2,
            TestFamily::AiThink,
            2,
            Some(creature),
            due,
        );
        assert_eq!(rejected, Err(OwnerTimerError::PendingLimitReached));
        assert_eq!(lane.pending_for_key(TestFamily::AiThink, Some(creature)), 1);
    }

    #[test]
    fn construction_rejects_a_family_cap_above_its_registered_maximum() {
        // The P2 concern: nothing stopped `for_generation` from accepting a cap above the
        // registered hard maximum (e.g. `[(AiThink, 2)]` when the registry caps it at 1).
        // Construction must refuse it rather than silently accept or clamp it.
        let gen1 = generation(1);
        let scope = scope_for(1);
        let rejected: Result<OwnerTimerLane<TestFamily, u64>, OwnerTimerError> =
            OwnerTimerLane::for_generation(
                scope,
                gen1,
                [(
                    TestFamily::AiThink,
                    FamilyPolicy {
                        max_pending: 2,
                        catch_up: CatchUpPolicy::SkipToLatest,
                    },
                )],
            );
        assert_eq!(
            rejected.err(),
            Some(OwnerTimerError::FamilyCapExceedsRegisteredMaximum)
        );
    }

    #[test]
    fn family_without_a_constructed_cap_fails_closed() {
        // A family the lane's owner never gave a policy to at construction can never hold a
        // pending timer (cap 0), rather than silently inheriting some other family's bound.
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(
            scope,
            gen1,
            [(
                TestFamily::Respawn,
                FamilyPolicy {
                    max_pending: 10,
                    catch_up: CatchUpPolicy::DeadlineState,
                },
            )],
        )
        .expect("cap within registered maximum");
        let mut owner_fence = fence(scope, gen1);
        let due = SemanticTimeMicros::from_micros(1_000);

        let stamp = issue_stamp(&mut owner_fence, gen1);
        let rejected = lane.schedule(&owner_fence, stamp, TestFamily::AiThink, 1, None, due);
        assert_eq!(rejected, Err(OwnerTimerError::PendingLimitReached));
    }

    #[test]
    fn duplicate_occurrence_identity_never_scheduled_twice() {
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let due = SemanticTimeMicros::from_micros(1_000);
        let stamp_1 = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(&owner_fence, stamp_1, TestFamily::Respawn, 7, None, due)
            .expect("first schedule");
        let stamp_2 = issue_stamp(&mut owner_fence, gen1);
        let rejected = lane.schedule(&owner_fence, stamp_2, TestFamily::Respawn, 7, None, due);
        assert_eq!(rejected, Err(OwnerTimerError::DuplicateOccurrence));
        assert_eq!(lane.pending_len(), 1);
    }

    #[test]
    fn cancel_removes_pending_occurrence() {
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let due = SemanticTimeMicros::from_micros(1_000);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(&owner_fence, stamp, TestFamily::Respawn, 1, None, due)
            .expect("schedule");
        assert!(lane.cancel(TestFamily::Respawn, 1));
        assert!(!lane.cancel(TestFamily::Respawn, 1));
        assert_eq!(lane.pending_len(), 0);

        let clock = VirtualOwnerClock::new(due);
        assert!(lane.drain_due(&clock, &owner_fence, |_| true).is_empty());
    }

    #[test]
    fn schedule_rejects_a_fence_that_never_matched_this_lane() {
        let gen1 = generation(1);
        let gen2 = generation(2);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut other_fence = fence(scope, gen2);
        let stamp = issue_stamp(&mut other_fence, gen2);
        let rejected = lane.schedule(
            &other_fence,
            stamp,
            TestFamily::AiThink,
            1,
            None,
            SemanticTimeMicros::from_micros(1),
        );
        assert_eq!(rejected, Err(OwnerTimerError::StaleOwnerGeneration));
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn schedule_rejects_a_fence_bound_to_a_different_scope_at_the_same_generation() {
        // The P1 fix: a fence proving the exact same generation number as the lane, but granted
        // for a *different* Channel, must never authorize this lane. Channel A's fence at
        // generation 1 must not authorize Channel B's lane, even though both happen to reach
        // generation 1 independently.
        let gen1 = generation(1);
        let channel_a = scope_for(1);
        let channel_b = scope_for(2);
        let mut lane = generous_lane(channel_a, gen1);
        let mut channel_b_fence = fence(channel_b, gen1);
        let stamp = issue_stamp(&mut channel_b_fence, gen1);
        let rejected = lane.schedule(
            &channel_b_fence,
            stamp,
            TestFamily::AiThink,
            1,
            None,
            SemanticTimeMicros::from_micros(1),
        );
        assert_eq!(rejected, Err(OwnerTimerError::StaleOwnerGeneration));
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn drain_due_fires_nothing_for_a_fence_that_never_matched_this_lane() {
        // A timer scheduled under the lane's own generation never fires when drained with a
        // fence proving a different generation: a superseded owner cannot pull mutation-shaped
        // input out of a lane it no longer owns.
        let gen1 = generation(1);
        let gen2 = generation(2);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let other_fence = fence(scope, gen2);
        let due = SemanticTimeMicros::from_micros(10);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(&owner_fence, stamp, TestFamily::AiThink, 1, None, due)
            .expect("schedule under gen1");

        let clock = VirtualOwnerClock::new(due);
        assert!(lane.drain_due(&clock, &other_fence, |_| true).is_empty());
        // The entry is untouched: the caller must build a fresh lane for gen2, not keep
        // draining this one.
        assert_eq!(lane.pending_len(), 1);
    }

    #[test]
    fn drain_due_fires_nothing_for_a_fence_bound_to_a_different_scope() {
        // The P1 fix, `drain_due` side: Channel B's fence at Channel A's exact generation number
        // must not let a superseded/foreign owner drain Channel A's lane.
        let gen1 = generation(1);
        let channel_a = scope_for(3);
        let channel_b = scope_for(4);
        let mut lane = generous_lane(channel_a, gen1);
        let mut owner_fence = fence(channel_a, gen1);
        let channel_b_fence = fence(channel_b, gen1);
        let due = SemanticTimeMicros::from_micros(10);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(&owner_fence, stamp, TestFamily::AiThink, 1, None, due)
            .expect("schedule under channel_a's fence");

        let clock = VirtualOwnerClock::new(due);
        assert!(
            lane.drain_due(&clock, &channel_b_fence, |_| true)
                .is_empty()
        );
        assert_eq!(lane.pending_len(), 1);
    }

    #[test]
    fn drain_due_requires_the_fence_to_still_be_current_after_a_real_handoff() {
        // The P1 fix: comparing two values that are both fixed at some earlier point (the
        // lane's own stored generation, and a caller-held raw generation) can never detect a
        // real handoff, because neither changes afterward. `owner_fence` must instead be
        // consulted live: it is the single mutated-in-place owner-cycle authority, so once a
        // real handoff advances it in place (`apply_external_grant`), a superseded owner still
        // holding this lane and a reference to that same fence observes the move and is
        // refused, even though `lane.generation()` and the fence's pre-handoff generation value
        // still trivially matched at schedule time.
        let gen1 = generation(1);
        let gen2 = generation(2);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let due = SemanticTimeMicros::from_micros(10);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(&owner_fence, stamp, TestFamily::AiThink, 1, None, due)
            .expect("schedule while the fence still proves gen1 current");

        // Channel ownership moves to gen2: the fence is advanced in place, exactly as the real
        // owner cycle does on handoff. The scope binding (still channel `scope`) is unchanged.
        owner_fence
            .apply_external_grant(gen2)
            .expect("advance fence to gen2");

        let clock = VirtualOwnerClock::new(due);
        let fired = lane.drain_due(&clock, &owner_fence, |_| true);
        assert!(fired.is_empty());
        // Nothing mutated: the superseded lane's pending entry is untouched.
        assert_eq!(lane.pending_len(), 1);
    }

    #[test]
    fn drain_due_drops_stale_target_without_returning_it() {
        // A timer from a stale target (actor-local) generation never fires: `target_is_current`
        // stands in for the owner's live-state check (FND-03 §10.3), since `ExactActorRef`
        // equality already binds identity and generation together and this generic lane cannot
        // (and must not) decode a foreign actor handle itself.
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let stale_actor = actor(12);
        let due = SemanticTimeMicros::from_micros(500);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp,
            TestFamily::AiThink,
            1,
            Some(stale_actor),
            due,
        )
        .expect("schedule");

        let clock = VirtualOwnerClock::new(due);
        // No live actor currently matches `stale_actor` (it respawned under a new generation).
        let fired = lane.drain_due(&clock, &owner_fence, |_| false);
        assert!(fired.is_empty());
        // The stale entry is consumed, not left pending forever.
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn drain_due_purges_not_yet_due_stale_target_without_returning_it() {
        // The P3 fix: a pending entry whose target has gone stale is purged as soon as a drain
        // observes it, even though its own deadline has not arrived yet, so death/respawn churn
        // on a target cannot accumulate a stale not-yet-due entry that would otherwise sit
        // forever (or fire later against a target it no longer matches).
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let stale_actor = actor(20);
        let not_due = SemanticTimeMicros::from_micros(10_000);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp,
            TestFamily::AiThink,
            1,
            Some(stale_actor),
            not_due,
        )
        .expect("schedule a not-yet-due entry for a target that will go stale");
        assert_eq!(lane.pending_len(), 1);

        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(1));
        // The target respawned under a new generation before its timer ever came due.
        let fired = lane.drain_due(&clock, &owner_fence, |_| false);
        assert!(fired.is_empty());
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn drain_due_keeps_current_target_and_ignores_others() {
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let live_actor = actor(13);
        let dead_actor = actor(14);
        let due = SemanticTimeMicros::from_micros(500);
        let stamp_1 = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp_1,
            TestFamily::AiThink,
            1,
            Some(live_actor),
            due,
        )
        .expect("schedule live");
        let stamp_2 = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp_2,
            TestFamily::Respawn,
            2,
            Some(dead_actor),
            due,
        )
        .expect("schedule dead");

        let clock = VirtualOwnerClock::new(due);
        let fired = lane.drain_due(&clock, &owner_fence, |target| target == live_actor);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].occurrence, 1);
    }

    #[test]
    fn skip_to_latest_reports_now_instead_of_the_stale_deadline_after_a_long_delay() {
        // The P2/catch-up fix: after a long delay past the original deadline, a SkipToLatest
        // timer still fires exactly once, and the fired timer reports the clock's current time
        // rather than the ancient original deadline, so a caller rescheduling relative to `due`
        // cannot be driven to replay every missed interval.
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = generous_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let creature = actor(40);
        let due = SemanticTimeMicros::from_micros(1_000);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp,
            TestFamily::AiThink,
            1,
            Some(creature),
            due,
        )
        .expect("schedule think timer");

        // The owner falls far behind: the clock jumps many missed intervals past the original
        // due time.
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(1_000_000));
        let fired = lane.drain_due(&clock, &owner_fence, |_| true);

        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].due, clock.now());
        assert_ne!(fired[0].due, due);
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn skip_to_latest_collapses_multiple_due_occurrences_for_the_same_key_into_one_fire() {
        // `drain_due` must never return more than one fired occurrence for one (family, target)
        // key in a single call under `SkipToLatest`, even when a family's registered ceiling
        // allows more than one concurrently pending entry and several of them came due at once.
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(
            scope,
            gen1,
            [(
                TestFamily::Respawn,
                FamilyPolicy {
                    max_pending: 2,
                    catch_up: CatchUpPolicy::SkipToLatest,
                },
            )],
        )
        .expect("cap within the supplied registered maximum");
        let mut owner_fence = fence(scope, gen1);
        let creature = actor(41);
        let earlier_due = SemanticTimeMicros::from_micros(1_000);
        let later_due = SemanticTimeMicros::from_micros(2_000);

        let stamp_a = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp_a,
            TestFamily::Respawn,
            1,
            Some(creature),
            earlier_due,
        )
        .expect("schedule earlier occurrence");
        let stamp_b = issue_stamp(&mut owner_fence, gen1);
        lane.schedule(
            &owner_fence,
            stamp_b,
            TestFamily::Respawn,
            2,
            Some(creature),
            later_due,
        )
        .expect("schedule later occurrence");

        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(5_000));
        let fired = lane.drain_due(&clock, &owner_fence, |_| true);

        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].occurrence, 2);
        assert_eq!(fired[0].due, clock.now());
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn skip_to_latest_replacement_keeps_deadline_order_across_keys() {
        // Key A is due at t1 and t3, key B at t2: after A's t1 entry is replaced by its t3
        // entry, B (t2) must still fire before A (t3).
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane: OwnerTimerLane<TestFamily, u64> = OwnerTimerLane::for_generation(
            scope,
            gen1,
            [(
                TestFamily::Respawn,
                FamilyPolicy {
                    max_pending: 2,
                    catch_up: CatchUpPolicy::SkipToLatest,
                },
            )],
        )
        .expect("cap within the registered maximum");
        let mut owner_fence = fence(scope, gen1);
        let (creature_a, creature_b) = (actor(41), actor(42));
        for (occurrence, target, due) in [
            (1, creature_a, 1_000),
            (3, creature_b, 2_000),
            (2, creature_a, 3_000),
        ] {
            let stamp = issue_stamp(&mut owner_fence, gen1);
            lane.schedule(
                &owner_fence,
                stamp,
                TestFamily::Respawn,
                occurrence,
                Some(target),
                SemanticTimeMicros::from_micros(due),
            )
            .expect("schedule");
        }

        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(5_000));
        let fired = lane.drain_due(&clock, &owner_fence, |_| true);

        let order: Vec<u64> = fired.iter().map(|timer| timer.occurrence).collect();
        assert_eq!(order, vec![3, 2]);
    }

    fn corpse(seed: u8) -> [u8; 16] {
        let mut bytes = uuid_v7(seed);
        bytes[0] = 2;
        bytes
    }

    fn decay_lane(
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> OwnerTimerLane<CorpseDecayFamily, CorpseDecayOccurrence> {
        OwnerTimerLane::for_generation(
            scope,
            generation,
            [(CorpseDecayFamily, CORPSE_DECAY_POLICY)],
        )
        .expect("decay cap within COMBAT01-CORPSES-PER-SCOPE")
    }

    #[test]
    fn corpse_decay_due_is_the_remaining_time_to_the_durable_deadline() {
        let now = SemanticTimeMicros::from_micros(5_000_000);
        // materialized_at 1_000 ms, deadline 61_000 ms, database clock at 1_500 ms.
        assert_eq!(
            corpse_decay_due(now, 1_500, 1_000 + CORPSE_DECAY_AFTER_MS),
            SemanticTimeMicros::from_micros(5_000_000 + 59_500_000)
        );
        // Exactly at the deadline, and long past it (a resumed or missed decay): due now.
        assert_eq!(corpse_decay_due(now, 61_000, 61_000), now);
        assert_eq!(corpse_decay_due(now, 900_000, 61_000), now);
        // Saturating: an absurd deadline never wraps into the past.
        assert_eq!(
            corpse_decay_due(SemanticTimeMicros::from_micros(u64::MAX - 1), 0, i64::MAX),
            SemanticTimeMicros::from_micros(u64::MAX)
        );
    }

    #[test]
    fn corpse_decay_fires_at_its_deadline_never_before_and_never_collapsed() {
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = decay_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let start = SemanticTimeMicros::from_micros(1_000);
        let due_a = corpse_decay_due(start, 0, CORPSE_DECAY_AFTER_MS);
        let due_b = corpse_decay_due(start, 0, CORPSE_DECAY_AFTER_MS + 1);
        for (seed, due) in [(1, due_a), (2, due_b)] {
            let stamp = issue_stamp(&mut owner_fence, gen1);
            schedule_corpse_decay(&mut lane, &owner_fence, stamp, corpse(seed), due)
                .expect("schedule decay");
        }
        let clock = VirtualOwnerClock::new(start);
        clock.advance(59_999_999);
        assert!(lane.drain_due(&clock, &owner_fence, |_| true).is_empty());
        clock.advance(1);
        let fired = lane.drain_due(&clock, &owner_fence, |_| true);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].occurrence.corpse_item_instance_id, corpse(1));
        assert_eq!(fired[0].due, due_a);
        // Long overdue: the second corpse still fires with its own deadline (DeadlineState).
        clock.advance(10_000_000);
        let fired = lane.drain_due(&clock, &owner_fence, |_| true);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].occurrence.corpse_item_instance_id, corpse(2));
        assert_eq!(fired[0].due, due_b);
        assert_eq!(lane.pending_len(), 0);
    }

    #[test]
    fn corpse_decay_cap_is_corpses_per_scope_and_a_corpse_is_scheduled_once() {
        assert_eq!(CorpseDecayFamily.registered_maximum(), 64);
        assert!(matches!(
            OwnerTimerLane::<CorpseDecayFamily, CorpseDecayOccurrence>::for_generation(
                scope_for(1),
                generation(1),
                [(
                    CorpseDecayFamily,
                    FamilyPolicy {
                        max_pending: COMBAT01_CORPSES_PER_SCOPE + 1,
                        catch_up: CatchUpPolicy::DeadlineState,
                    },
                )],
            ),
            Err(OwnerTimerError::FamilyCapExceedsRegisteredMaximum)
        ));
        let gen1 = generation(1);
        let scope = scope_for(1);
        let mut lane = decay_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let due = SemanticTimeMicros::from_micros(10);
        for seed in 0..64_u8 {
            let stamp = issue_stamp(&mut owner_fence, gen1);
            schedule_corpse_decay(&mut lane, &owner_fence, stamp, corpse(seed), due)
                .expect("up to COMBAT01-CORPSES-PER-SCOPE");
        }
        let stamp = issue_stamp(&mut owner_fence, gen1);
        assert_eq!(
            schedule_corpse_decay(&mut lane, &owner_fence, stamp, corpse(64), due),
            Err(OwnerTimerError::PendingLimitReached)
        );
        assert_eq!(lane.pending_len(), 64);
        let mut lane = decay_lane(scope, gen1);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        schedule_corpse_decay(&mut lane, &owner_fence, stamp, corpse(1), due).expect("first");
        let stamp = issue_stamp(&mut owner_fence, gen1);
        assert_eq!(
            schedule_corpse_decay(&mut lane, &owner_fence, stamp, corpse(1), due),
            Err(OwnerTimerError::DuplicateOccurrence)
        );
    }

    #[test]
    fn a_new_owner_generation_reschedules_decay_into_a_fresh_lane_only() {
        // The superseded owner's lane fires nothing after a handoff; the new owner rebuilds its
        // lane from the durable recovery query (here: one corpse already past its deadline).
        let gen1 = generation(1);
        let gen2 = generation(2);
        let scope = scope_for(1);
        let mut old_lane = decay_lane(scope, gen1);
        let mut owner_fence = fence(scope, gen1);
        let start = SemanticTimeMicros::from_micros(0);
        let stamp = issue_stamp(&mut owner_fence, gen1);
        schedule_corpse_decay(&mut old_lane, &owner_fence, stamp, corpse(1), start)
            .expect("schedule under gen1");
        owner_fence
            .apply_external_grant(gen2)
            .expect("handoff to gen2");
        let clock = VirtualOwnerClock::new(start);
        assert!(
            old_lane
                .drain_due(&clock, &owner_fence, |_| true)
                .is_empty()
        );

        let mut new_lane = decay_lane(scope, gen2);
        let stamp = issue_stamp(&mut owner_fence, gen2);
        let due = corpse_decay_due(clock.now(), 120_000, 61_000);
        schedule_corpse_decay(&mut new_lane, &owner_fence, stamp, corpse(1), due)
            .expect("reschedule under gen2");
        let fired = new_lane.drain_due(&clock, &owner_fence, |_| true);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].occurrence.corpse_item_instance_id, corpse(1));
    }
}
