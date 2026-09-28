//! Rat behaviour: think -> perceive -> decide -> act (D53, D115).
//!
//! GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1 §4.4
//! (`docs/architecture/reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md`),
//! owner-reallocated to this task as "AI-2 rat behaviour" by the live #162 control-plane
//! allocation (issuecomment-5879863781), with D115's numeric values
//! (issuecomment-5879404970): think every 1000 ms, perception 7 tiles, 25% wander chance with
//! radius 2, respawn 60 s / 3 retries every 5 s (respawn/spawn realization stays out of this
//! slice's scope; D116's two-rat spawn is content, routed to AI-3).
//!
//! This module is foundation-self-contained by design (only `super::*` and the external
//! `oteryn_simulation_determinism`/`sha2` crates), exactly like `owner_timer.rs`: several
//! integration test binaries (`tests/wp5_s3b_composition.rs` and siblings) path-include
//! `foundation/mod.rs` as an isolated crate root without `crate::movement` or `crate::content`,
//! so this module never imports either. `decide` returns a [`CardinalDirection`], a small
//! Movement-independent cardinal enum; the caller (a future `ai/**`/`lib.rs` wiring stage, or a
//! test in `movement.rs` that already has `crate::movement` in scope) maps it onto
//! `crate::movement::CardinalStep` and applies it through the existing, unmodified
//! `crate::movement::step_cardinal`/`MovementOwnerTurn` (§4.5: "it submits one step ... through
//! the Movement owner turn"). `runtime_actor_carrier.rs`'s `read_movement_position`/
//! `commit_movement_position` no longer reject a `CreatureOccupied` slot (this task's other
//! change), so that existing, unmodified Movement path already serves a creature actor
//! generically; nothing here bypasses it or writes a position directly (the explicitly rejected
//! option, §6: "AI writes positions or damage directly").
//!
//! **Bite / player damage (§4.6, §4.7).** No player vitals (HP) exist anywhere in this codebase
//! yet (`spell` step P3b-2, routed to AI-4's typed-issuer Ability wiring, per the decision's own
//! §3 UNKNOWN and §4.7: "the bite may be exercised only against a test vitals fixture, never as a
//! shipped behaviour"). [`decide`] fully computes the Attack decision (adjacency, legality,
//! cooldown, the deterministic chance draw and the deterministic damage roll), and
//! [`BiteCommitter`] is the injected, owner-applied commit boundary GAME-AI-01 §3 requires ("one
//! current Channel owner ... commits"; AI never commits itself). This slice's only
//! `BiteCommitter` is the bounded `#[cfg(test)]` vitals fixture in this module's tests, exactly
//! matching §4.7's stated allowance; AI-4 supplies the real Ability-mediated implementation once
//! player vitals exist.
//!
//! **Binding items carried over from AI-1** (`docs/agents/tasks/archive/OTV2-20260928-ai1-owner-timer-lane.md`):
//! - (a) bounded occurrence-identity replay evidence: [`RatThinkState::next_sequence`] is a
//!   monotonically increasing (`saturating_add`, never decremented or reset within a creature
//!   generation) per-creature counter; [`schedule_next_think`] is the only place a
//!   [`ThinkOccurrence`] is minted, always from this counter's current value before advancing it.
//!   Since `ExactActorRef` embeds the creature's actor-local generation and generations are never
//!   reused (D52; `runtime_actor_carrier.rs`'s `commit_creature_damage_inner`/admission), the full
//!   `(creature, sequence)` identity can never repeat within this lane's lifetime: the evidence
//!   is this one `u64` high-water mark, O(1) per live creature, never an unbounded log.
//! - (b) unforgeable `RuntimeWorkStamp` provenance: already enforced by `OwnerTimerLane::schedule`
//!   itself (`accepts_stamp`); this module never fabricates a stamp, only forwards one the caller
//!   obtained from `ScopeRuntimeFence::accept_input` + `stamp`, exactly as `schedule_next_think`'s
//!   signature requires.

#[cfg(test)]
use super::owner_timer::VirtualOwnerClock;
use super::owner_timer::{
    AI01_PENDING_TIMERS_PER_ACTOR, CatchUpPolicy, FamilyPolicy, OwnerTimerError, OwnerTimerLane,
    SemanticTimeMicros, TimerFamily,
};
use super::{ExactActorRef, RuntimeWorkStamp, ScopeRuntimeFence};
use oteryn_simulation_determinism::{
    DecisionError, DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

/// AI01-PERCEPTION-CANDIDATES (`RESOURCE_LIMITS_REGISTRY.json`), registered by the bootstrap
/// slice: at most this many perception candidates per decision. `decide` rejects more before
/// doing any other work.
pub(crate) const AI01_PERCEPTION_CANDIDATES: usize = 64;

/// One local cardinal direction, independent of `crate::movement::CardinalStep` (see module doc:
/// this module never imports `crate::movement`). `(dx, dy)` matches `CardinalStep::delta` exactly
/// so a caller's mapping is a trivial 1:1 match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardinalDirection {
    North,
    East,
    South,
    West,
}

impl CardinalDirection {
    const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    const fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }
}

/// A local tile position, independent of `crate::movement::MovementLocalPosition` for the same
/// reason as [`CardinalDirection`]. Field-for-field identical; a caller converts trivially.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RatLocalPosition {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) floor: i16,
}

impl RatLocalPosition {
    const fn apply(self, direction: CardinalDirection) -> Option<Self> {
        let (dx, dy) = direction.delta();
        let Some(x) = self.x.checked_add(dx) else {
            return None;
        };
        let Some(y) = self.y.checked_add(dy) else {
            return None;
        };
        Some(Self {
            x,
            y,
            floor: self.floor,
        })
    }

    /// Chebyshev (tile-radius) distance: the perception/wander-radius metric §4.4/D115 use.
    /// Returns `None` on a different floor (never perceived/bounded across floors).
    fn chebyshev(self, other: Self) -> Option<i64> {
        if self.floor != other.floor {
            return None;
        }
        let dx = i64::from(self.x) - i64::from(other.x);
        let dy = i64::from(self.y) - i64::from(other.y);
        Some(dx.abs().max(dy.abs()))
    }

    /// One of the four Movement cardinal steps away (distance exactly 1, same floor); adjacency
    /// as `commit_cardinal`'s own east/west/south/north test defines it, not diagonal Chebyshev-1.
    fn is_cardinally_adjacent(self, other: Self) -> bool {
        self.floor == other.floor
            && ((self.x == other.x && (self.y - other.y).abs() == 1)
                || (self.y == other.y && (self.x - other.x).abs() == 1))
    }
}

/// D115 + the rat's Reference bite evidence (§4.8: content-declared, never hardcoded by the
/// decision logic itself; these are the caller-supplied values, the accepted D115/Reference
/// numbers only by convention in tests and the eventual content record).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RatBehaviorDefinition {
    /// D115: 1000 ms.
    pub(crate) think_interval_micros: u64,
    /// D115: 7 tiles.
    pub(crate) perception_range_tiles: i64,
    /// D115: 25% = 250 per mille.
    pub(crate) wander_chance_per_mille: u16,
    /// D115: 2 tiles, measured from `home` (the spawn cell, §4.4/§4.3).
    pub(crate) wander_radius_tiles: i64,
    /// Reference evidence (task prompt, not yet a numbered owner decision): 2000 ms.
    pub(crate) bite_interval_micros: u64,
    /// Content input (§4.8): no owner/Reference value routed yet.
    pub(crate) bite_chance_per_mille: u16,
    /// Reference evidence: 0.
    pub(crate) bite_damage_min: i64,
    /// Reference evidence: 8.
    pub(crate) bite_damage_max: i64,
}

/// One perceived player candidate (§4.4: "players within the definition's perception range in
/// the same Channel scope"). `legal_attack_target` is caller-derived (alive, not inside the PvE
/// re-entry protection window, GAME-AI-01 §9): this module decodes no foreign owner facts itself,
/// exactly as `OwnerTimerLane::drain_due`'s `target_is_current` closure does for staleness.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PerceivedPlayer {
    pub(crate) actor: ExactActorRef,
    pub(crate) position: RatLocalPosition,
    pub(crate) legal_attack_target: bool,
}

/// The bounded outcome of one think resolution (§4.4: "a resolution proposes at most one
/// action").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RatDecision {
    /// Bite chance succeeded against an adjacent, legal, off-cooldown target.
    Attack { target: ExactActorRef },
    /// A perceived target exists; one step toward it, revalidated by the caller's Movement
    /// commit before use (§4.5).
    Chase {
        step: CardinalDirection,
        target: ExactActorRef,
    },
    /// No target perceived; the wander-chance draw succeeded and a radius-bounded step exists.
    Wander { step: CardinalDirection },
    /// A failed bite-chance draw (§4.4.1: ends idle, never falls through to chase/wander), no
    /// perceived target and a failed/impossible wander, or an adjacent-but-ineligible target
    /// (cooling down or protection-shielded) with no other perceived candidate to chase toward.
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RatThinkError {
    /// AI01-PERCEPTION-CANDIDATES: more candidates than the registered ceiling were supplied.
    PerceptionCandidatesExceeded,
    Decision(DecisionError),
}

impl From<DecisionError> for RatThinkError {
    fn from(error: DecisionError) -> Self {
        Self::Decision(error)
    }
}

impl std::fmt::Display for RatThinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PerceptionCandidatesExceeded => {
                formatter.write_str("perception candidates exceed the registered ceiling")
            }
            Self::Decision(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for RatThinkError {}

/// Maps a uniform `u64` draw onto `0..1000` by multiply-shift (no modulo bias), matching
/// `domain::death::per_mille_roll`'s established technique in this codebase.
fn per_mille_roll(draw: u64) -> u16 {
    u16::try_from((u128::from(draw) * 1_000) >> 64).unwrap_or(u16::MAX)
}

/// Maps a uniform `u64` draw onto an unbiased index `0..len` (`len` in `1..=4` here).
fn index_roll(draw: u64, len: usize) -> usize {
    let len_u128 = u128::try_from(len).unwrap_or(1);
    usize::try_from((u128::from(draw) * len_u128) >> 64).unwrap_or(len.saturating_sub(1))
}

/// One bounded, deterministic think resolution (§4.4). Pure: it reads only its arguments and
/// proposes at most one action; nothing is committed here (GAME-AI-01 §3: only the owner
/// commits).
pub(crate) fn decide(
    definition: &RatBehaviorDefinition,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    self_position: RatLocalPosition,
    home: RatLocalPosition,
    bite_ready: bool,
    candidates: &[PerceivedPlayer],
) -> Result<RatDecision, RatThinkError> {
    if candidates.len() > AI01_PERCEPTION_CANDIDATES {
        return Err(RatThinkError::PerceptionCandidatesExceeded);
    }

    let nearest = candidates
        .iter()
        .filter_map(|candidate| {
            self_position
                .chebyshev(candidate.position)
                .filter(|distance| *distance <= definition.perception_range_tiles)
                .map(|distance| (distance, candidate))
        })
        .min_by_key(|(distance, candidate)| (*distance, candidate.actor.placement_identity()));

    if let Some((_, target)) = nearest {
        if target.legal_attack_target
            && bite_ready
            && self_position.is_cardinally_adjacent(target.position)
        {
            let roll =
                deterministic_decision_u64(root, occurrence, "oteryn.ai.rat.bite.chance.v1", 0)?;
            return Ok(if per_mille_roll(roll) < definition.bite_chance_per_mille {
                RatDecision::Attack {
                    target: target.actor,
                }
            } else {
                // §4.4.1: a failed draw ends the think idle; it never chases or wanders.
                RatDecision::Idle
            });
        }
        return Ok(step_toward(self_position, target.position)
            .map(|step| RatDecision::Chase {
                step,
                target: target.actor,
            })
            .unwrap_or(RatDecision::Idle));
    }

    let chance_roll =
        deterministic_decision_u64(root, occurrence, "oteryn.ai.rat.wander.chance.v1", 0)?;
    if per_mille_roll(chance_roll) >= definition.wander_chance_per_mille {
        return Ok(RatDecision::Idle);
    }
    let valid_steps: Vec<CardinalDirection> = CardinalDirection::ALL
        .into_iter()
        .filter(|direction| {
            self_position
                .apply(*direction)
                .and_then(|next| next.chebyshev(home))
                .is_some_and(|distance| distance <= definition.wander_radius_tiles)
        })
        .collect();
    if valid_steps.is_empty() {
        return Ok(RatDecision::Idle);
    }
    let direction_roll =
        deterministic_decision_u64(root, occurrence, "oteryn.ai.rat.wander.direction.v1", 0)?;
    let index = index_roll(direction_roll, valid_steps.len());
    Ok(RatDecision::Wander {
        step: valid_steps[index],
    })
}

/// One cardinal step reducing distance to `target`: the axis with the larger absolute delta
/// moves first, ties broken toward the x axis (an arbitrary but fixed, deterministic rule; no
/// production pathfinding algorithm is decided here, §7). `None` only when `self_position` and
/// `target` already coincide or are off-floor (never perceived as a candidate in that case).
fn step_toward(
    self_position: RatLocalPosition,
    target: RatLocalPosition,
) -> Option<CardinalDirection> {
    if self_position.floor != target.floor {
        return None;
    }
    let dx = i64::from(target.x) - i64::from(self_position.x);
    let dy = i64::from(target.y) - i64::from(self_position.y);
    if dx == 0 && dy == 0 {
        return None;
    }
    if dx.abs() >= dy.abs() && dx != 0 {
        return Some(if dx > 0 {
            CardinalDirection::East
        } else {
            CardinalDirection::West
        });
    }
    if dy != 0 {
        return Some(if dy > 0 {
            CardinalDirection::South
        } else {
            CardinalDirection::North
        });
    }
    None
}

/// Roll the bite's damage magnitude (Reference evidence: uniform 0-8), deterministically bound
/// to the think occurrence. Called only after `decide` returns `RatDecision::Attack`.
pub(crate) fn roll_bite_damage(
    definition: &RatBehaviorDefinition,
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
) -> Result<i64, RatThinkError> {
    let span = u64::try_from(definition.bite_damage_max - definition.bite_damage_min)
        .unwrap_or(0)
        .saturating_add(1);
    let roll = deterministic_decision_u64(root, occurrence, "oteryn.ai.rat.bite.damage.v1", 0)?;
    let offset = i64::try_from(roll.checked_rem(span).unwrap_or(0)).unwrap_or(0);
    Ok(definition.bite_damage_min.saturating_add(offset))
}

/// D54: creature damage never takes a player's HP below 1 in this slice.
pub(crate) const PLAYER_HP_FLOOR: i64 = 1;

/// The owner-applied commit boundary for one bite occurrence (GAME-AI-01 §3: AI proposes, the
/// owner commits; see module doc for why this slice has no production implementation). An
/// identical retry of the same `ThinkOccurrence` must return the first result rather than
/// applying damage twice (§4.6).
pub(crate) trait BiteCommitter {
    type Error;

    fn commit_bite(
        &mut self,
        target: ExactActorRef,
        occurrence: ThinkOccurrence,
        damage: i64,
    ) -> Result<i64, Self::Error>;
}

/// AI-RL-06 / `AI01-PENDING-TIMERS-PER-ACTOR`: the rat slice's one timer family. `OwnerTimerLane`
/// is fully generic (AI-1, `owner_timer.rs`); this is the concrete `Family` this task supplies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RatTimerFamily {
    Think,
}

impl TimerFamily for RatTimerFamily {
    fn registered_maximum(self) -> usize {
        match self {
            Self::Think => AI01_PENDING_TIMERS_PER_ACTOR,
        }
    }
}

/// A think occurrence identity (§4.2: "a think occurrence is (creature `ExactActorRef`,
/// per-actor think sequence number)"). See the module doc's binding item (a) for why this can
/// never repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThinkOccurrence {
    pub(crate) creature: ExactActorRef,
    pub(crate) sequence: u64,
}

/// The lane policy this slice's think family uses (§4.9: `SKIP_TO_LATEST`, at most one pending
/// per creature).
#[must_use]
pub(crate) const fn think_family_policy() -> FamilyPolicy {
    FamilyPolicy {
        max_pending: AI01_PENDING_TIMERS_PER_ACTOR,
        catch_up: CatchUpPolicy::SkipToLatest,
    }
}

/// Per-creature, process-local, ephemeral think bookkeeping (§4.2: "Both timers are process-local.
/// They need no durable state"). Not carrier state: this is owner-cycle-local scheduling/cooldown
/// bookkeeping, distinct from the carrier's own identity/position/health facts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RatThinkState {
    /// Binding item (a)'s bounded replay evidence: see the module doc. Only ever read by
    /// `schedule_next_think` and then advanced; never reset except by constructing a fresh state
    /// for a fresh creature generation.
    next_sequence: u64,
    last_bite_committed_at: Option<SemanticTimeMicros>,
    home: RatLocalPosition,
}

impl RatThinkState {
    #[must_use]
    pub(crate) const fn new(home: RatLocalPosition) -> Self {
        Self {
            next_sequence: 0,
            last_bite_committed_at: None,
            home,
        }
    }

    #[must_use]
    pub(crate) const fn home(&self) -> RatLocalPosition {
        self.home
    }

    #[must_use]
    pub(crate) const fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// The bite interval has elapsed (or no bite was ever committed): §4.6, "the bite's interval
    /// has elapsed".
    #[must_use]
    pub(crate) fn bite_ready(
        &self,
        definition: &RatBehaviorDefinition,
        now: SemanticTimeMicros,
    ) -> bool {
        self.last_bite_committed_at.is_none_or(|last| {
            now.get().saturating_sub(last.get()) >= definition.bite_interval_micros
        })
    }

    pub(crate) fn record_bite_committed(&mut self, now: SemanticTimeMicros) {
        self.last_bite_committed_at = Some(now);
    }
}

/// Schedules the next think timer for `creature` and returns the [`ThinkOccurrence`] identity it
/// was scheduled under (§4.2, §4.4: "the next think timer is scheduled from the definition's
/// think interval whatever the outcome"). `owner_fence`/`scheduling_stamp` must be the caller's
/// live fence and a stamp it issued (binding item (b); already enforced by
/// `OwnerTimerLane::schedule` itself).
pub(crate) fn schedule_next_think(
    lane: &mut OwnerTimerLane<RatTimerFamily, ThinkOccurrence>,
    owner_fence: &ScopeRuntimeFence,
    scheduling_stamp: RuntimeWorkStamp,
    creature: ExactActorRef,
    state: &mut RatThinkState,
    definition: &RatBehaviorDefinition,
    now: SemanticTimeMicros,
) -> Result<ThinkOccurrence, OwnerTimerError> {
    let occurrence = ThinkOccurrence {
        creature,
        sequence: state.next_sequence,
    };
    let due = now.saturating_add_micros(definition.think_interval_micros);
    lane.schedule(
        owner_fence,
        scheduling_stamp,
        RatTimerFamily::Think,
        occurrence,
        Some(creature),
        due,
    )?;
    state.next_sequence = state.next_sequence.saturating_add(1);
    Ok(occurrence)
}

/// Derives this occurrence's [`DecisionOccurrenceId`] (§4.4: "a SIM RNG stream bound to the think
/// occurrence"; a retry of the same occurrence never redraws because this derivation and every
/// `deterministic_decision_u64` draw are pure functions of it).
#[must_use]
pub(crate) fn occurrence_decision_id(occurrence: ThinkOccurrence) -> DecisionOccurrenceId {
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"oteryn.ai.rat.think-occurrence.v1")
        .chain_update(occurrence.creature.placement_identity())
        .chain_update(occurrence.sequence.to_be_bytes())
        .finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    DecisionOccurrenceId::from_bytes(bytes)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId};

    fn root(seed: u8) -> GameplayDecisionRoot {
        GameplayDecisionRoot::from_bytes([seed; 32])
    }

    fn pos(x: i32, y: i32) -> RatLocalPosition {
        RatLocalPosition { x, y, floor: 7 }
    }

    fn uuid_v7(seed: u8) -> [u8; 16] {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    }

    fn actor(seed: u8) -> ExactActorRef {
        ExactActorRef::transport_fixture(
            WorldId::decode(&uuid_v7(seed)).expect("world"),
            ChannelId::decode(&uuid_v7(seed.wrapping_add(1))).expect("channel"),
        )
    }

    fn definition() -> RatBehaviorDefinition {
        // D115 + the Reference bite evidence cited by the task; not an owner-decided bite
        // chance (§4.8: content input), so tests fix one for determinism.
        RatBehaviorDefinition {
            think_interval_micros: 1_000_000,
            perception_range_tiles: 7,
            wander_chance_per_mille: 250,
            wander_radius_tiles: 2,
            bite_interval_micros: 2_000_000,
            bite_chance_per_mille: 500,
            bite_damage_min: 0,
            bite_damage_max: 8,
        }
    }

    fn occurrence_for(sequence: u64) -> DecisionOccurrenceId {
        occurrence_decision_id(ThinkOccurrence {
            creature: actor(1),
            sequence,
        })
    }

    #[test]
    fn perception_candidates_over_the_registered_ceiling_are_rejected() {
        let candidates: Vec<PerceivedPlayer> = (0..=AI01_PERCEPTION_CANDIDATES)
            .map(|index| PerceivedPlayer {
                actor: actor(u8::try_from(index % 250).unwrap_or(0)),
                position: pos(0, 0),
                legal_attack_target: true,
            })
            .collect();
        let result = decide(
            &definition(),
            &root(1),
            occurrence_for(0),
            pos(0, 0),
            pos(0, 0),
            true,
            &candidates,
        );
        assert_eq!(result, Err(RatThinkError::PerceptionCandidatesExceeded));
    }

    #[test]
    fn adjacent_legal_ready_target_may_attack_or_go_idle_never_chase_or_wander() {
        let candidates = [PerceivedPlayer {
            actor: actor(2),
            position: pos(1, 0),
            legal_attack_target: true,
        }];
        // Sweep enough occurrences to observe both a hit and a miss of the 50% bite chance.
        let mut saw_attack = false;
        let mut saw_idle = false;
        for sequence in 0..64 {
            let decision = decide(
                &definition(),
                &root(3),
                occurrence_for(sequence),
                pos(0, 0),
                pos(0, 0),
                true,
                &candidates,
            )
            .expect("decide succeeds");
            assert!(
                matches!(decision, RatDecision::Attack { .. } | RatDecision::Idle),
                "adjacent+legal+ready must only attack or idle, got {decision:?}"
            );
            if let RatDecision::Attack { target } = decision {
                assert_eq!(target, actor(2));
                saw_attack = true;
            } else {
                saw_idle = true;
            }
        }
        assert!(saw_attack, "expected at least one successful bite draw");
        assert!(saw_idle, "expected at least one failed bite draw");
    }

    #[test]
    fn bite_on_cooldown_chases_instead_of_attacking() {
        let candidates = [PerceivedPlayer {
            actor: actor(4),
            position: pos(1, 0),
            legal_attack_target: true,
        }];
        let decision = decide(
            &definition(),
            &root(5),
            occurrence_for(0),
            pos(0, 0),
            pos(0, 0),
            false, // bite not ready
            &candidates,
        )
        .expect("decide succeeds");
        // Adjacent (distance 1) chase still resolves to a step: `step_toward` picks x first.
        assert_eq!(
            decision,
            RatDecision::Chase {
                step: CardinalDirection::East,
                target: actor(4),
            }
        );
    }

    #[test]
    fn re_entry_protected_target_is_chased_but_never_bitten() {
        let candidates = [PerceivedPlayer {
            actor: actor(6),
            position: pos(1, 0),
            legal_attack_target: false,
        }];
        for sequence in 0..16 {
            let decision = decide(
                &definition(),
                &root(7),
                occurrence_for(sequence),
                pos(0, 0),
                pos(0, 0),
                true,
                &candidates,
            )
            .expect("decide succeeds");
            assert!(
                !matches!(decision, RatDecision::Attack { .. }),
                "a protected target must never be bitten"
            );
        }
    }

    #[test]
    fn perceived_but_not_adjacent_target_chases_toward_it() {
        let candidates = [PerceivedPlayer {
            actor: actor(8),
            position: pos(3, 5),
            legal_attack_target: true,
        }];
        let decision = decide(
            &definition(),
            &root(9),
            occurrence_for(0),
            pos(0, 0),
            pos(0, 0),
            true,
            &candidates,
        )
        .expect("decide succeeds");
        // dy (5) dominates dx (3): the y axis moves first.
        assert_eq!(
            decision,
            RatDecision::Chase {
                step: CardinalDirection::South,
                target: actor(8),
            }
        );
    }

    #[test]
    fn nearest_wins_ties_broken_by_stable_identity() {
        let a = actor(10);
        let b = actor(12);
        let (first, second) = if a.placement_identity() < b.placement_identity() {
            (a, b)
        } else {
            (b, a)
        };
        let candidates = [
            PerceivedPlayer {
                actor: second,
                position: pos(2, 0),
                legal_attack_target: true,
            },
            PerceivedPlayer {
                actor: first,
                position: pos(-2, 0),
                legal_attack_target: true,
            },
        ];
        let decision = decide(
            &definition(),
            &root(11),
            occurrence_for(0),
            pos(0, 0),
            pos(0, 0),
            true,
            &candidates,
        )
        .expect("decide succeeds");
        assert_eq!(
            decision,
            RatDecision::Chase {
                step: CardinalDirection::West,
                target: first,
            },
            "both candidates are equidistant; the lower stable identity must win"
        );
    }

    #[test]
    fn no_target_wanders_bounded_by_radius_or_stays_idle() {
        let home = pos(0, 0);
        for sequence in 0..64 {
            let decision = decide(
                &definition(),
                &root(13),
                occurrence_for(sequence),
                pos(2, 0), // already at the radius-2 boundary east of home
                home,
                true,
                &[],
            )
            .expect("decide succeeds");
            assert!(
                matches!(decision, RatDecision::Wander { .. } | RatDecision::Idle),
                "expected wander or idle, got {decision:?}"
            );
            if let RatDecision::Wander { step } = decision {
                let next = pos(2, 0)
                    .apply(step)
                    .expect("no coordinate overflow in this fixture");
                assert!(
                    next.chebyshev(home).expect("same floor") <= 2,
                    "a wander step must never leave the definition's radius"
                );
                assert_ne!(step, CardinalDirection::East, "east would leave the radius");
            }
        }
    }

    #[test]
    fn decide_is_deterministic_and_a_retry_never_redraws() {
        let candidates = [PerceivedPlayer {
            actor: actor(14),
            position: pos(1, 0),
            legal_attack_target: true,
        }];
        let first = decide(
            &definition(),
            &root(15),
            occurrence_for(9),
            pos(0, 0),
            pos(0, 0),
            true,
            &candidates,
        )
        .expect("decide succeeds");
        let retry = decide(
            &definition(),
            &root(15),
            occurrence_for(9),
            pos(0, 0),
            pos(0, 0),
            true,
            &candidates,
        )
        .expect("decide succeeds");
        assert_eq!(first, retry);
    }

    #[test]
    fn identical_snapshot_gives_identical_decision_under_shuffled_candidate_order() {
        let candidates_forward = [
            PerceivedPlayer {
                actor: actor(16),
                position: pos(5, 5),
                legal_attack_target: true,
            },
            PerceivedPlayer {
                actor: actor(18),
                position: pos(1, 0),
                legal_attack_target: true,
            },
        ];
        let candidates_reversed = [candidates_forward[1], candidates_forward[0]];
        let forward = decide(
            &definition(),
            &root(17),
            occurrence_for(0),
            pos(0, 0),
            pos(0, 0),
            true,
            &candidates_forward,
        )
        .expect("decide succeeds");
        let reversed = decide(
            &definition(),
            &root(17),
            occurrence_for(0),
            pos(0, 0),
            pos(0, 0),
            true,
            &candidates_reversed,
        )
        .expect("decide succeeds");
        assert_eq!(forward, reversed);
    }

    #[test]
    fn bite_damage_roll_stays_within_the_reference_range_and_is_deterministic() {
        let definition = definition();
        for sequence in 0..64 {
            let occurrence = occurrence_for(sequence);
            let first =
                roll_bite_damage(&definition, &root(19), occurrence).expect("roll succeeds");
            let retry =
                roll_bite_damage(&definition, &root(19), occurrence).expect("roll succeeds");
            assert_eq!(first, retry);
            assert!(
                (definition.bite_damage_min..=definition.bite_damage_max).contains(&first),
                "damage {first} outside [{}, {}]",
                definition.bite_damage_min,
                definition.bite_damage_max
            );
        }
    }

    /// Bounded, `#[cfg(test)]`-only vitals fixture (§4.7: "the bite may be exercised only
    /// against a test vitals fixture, never as a shipped behaviour"). Idempotent per occurrence:
    /// a retry of the same `ThinkOccurrence` returns the memoized result rather than applying
    /// damage a second time (§4.6), bounded to the single most recent occurrence (the same
    /// bound `OwnerTimerLane`'s `SkipToLatest` gives the think family itself: only the latest
    /// occurrence is ever retried before a fresh one is scheduled).
    struct TestPlayerVitals {
        hp: i64,
        last_occurrence: Option<(ThinkOccurrence, i64)>,
    }

    impl TestPlayerVitals {
        const fn new(hp: i64) -> Self {
            Self {
                hp,
                last_occurrence: None,
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct TestVitalsError;

    impl BiteCommitter for TestPlayerVitals {
        type Error = TestVitalsError;

        fn commit_bite(
            &mut self,
            _target: ExactActorRef,
            occurrence: ThinkOccurrence,
            damage: i64,
        ) -> Result<i64, Self::Error> {
            if let Some((recorded, hp_after)) = self.last_occurrence
                && recorded == occurrence
            {
                return Ok(hp_after);
            }
            self.hp = (self.hp - damage).max(PLAYER_HP_FLOOR);
            self.last_occurrence = Some((occurrence, self.hp));
            Ok(self.hp)
        }
    }

    #[test]
    fn bite_commit_clamps_to_the_d54_floor_and_never_applies_twice() {
        let mut vitals = TestPlayerVitals::new(3);
        let target = actor(20);
        let occurrence = ThinkOccurrence {
            creature: actor(1),
            sequence: 0,
        };
        let first = vitals
            .commit_bite(target, occurrence, 8)
            .expect("commit succeeds");
        assert_eq!(first, PLAYER_HP_FLOOR, "3 - 8 clamps to the D54 floor of 1");
        // A retry of the exact same occurrence must return the memoized result, not apply
        // another 8 damage (which would otherwise still be floored, so use a value that would
        // reveal a double-apply if it happened).
        let retry = vitals
            .commit_bite(target, occurrence, 8)
            .expect("commit succeeds");
        assert_eq!(retry, first);
        // A fresh occurrence applies fresh damage.
        let next_occurrence = ThinkOccurrence {
            creature: actor(1),
            sequence: 1,
        };
        let mut fresh = TestPlayerVitals::new(10);
        let applied = fresh
            .commit_bite(target, next_occurrence, 4)
            .expect("commit succeeds");
        assert_eq!(applied, 6);
    }

    fn scope(channel_seed: u8) -> RuntimeScopeRefV1 {
        RuntimeScopeRefV1::channel(
            WorldId::decode(&uuid_v7(30)).expect("world"),
            ChannelId::decode(&uuid_v7(channel_seed)).expect("channel"),
        )
    }

    #[test]
    fn schedule_next_think_advances_the_bounded_sequence_and_never_repeats() {
        let generation = ScopeOwnershipGeneration::new(1).expect("nonzero generation");
        let channel_scope = scope(31);
        let mut lane: OwnerTimerLane<RatTimerFamily, ThinkOccurrence> =
            OwnerTimerLane::for_generation(
                channel_scope,
                generation,
                [(RatTimerFamily::Think, think_family_policy())],
            )
            .expect("cap within its registered maximum");
        let mut owner_fence =
            ScopeRuntimeFence::from_external_grant(generation).with_scope(channel_scope);
        let creature = actor(32);
        let mut state = RatThinkState::new(pos(0, 0));
        let now = SemanticTimeMicros::from_micros(0);

        let ordinal = owner_fence
            .accept_input(generation)
            .expect("issue a live ordinal");
        let stamp = owner_fence.stamp(ordinal);
        let first = schedule_next_think(
            &mut lane,
            &owner_fence,
            stamp,
            creature,
            &mut state,
            &definition(),
            now,
        )
        .expect("schedule succeeds");
        assert_eq!(first.sequence, 0);
        assert_eq!(state.next_sequence(), 1);

        // AI01-PENDING-TIMERS-PER-ACTOR = 1: a second think cannot be pending at once until the
        // first drains, exactly like AI-1's own coverage of this cap.
        let ordinal_2 = owner_fence
            .accept_input(generation)
            .expect("issue a live ordinal");
        let stamp_2 = owner_fence.stamp(ordinal_2);
        let rejected = schedule_next_think(
            &mut lane,
            &owner_fence,
            stamp_2,
            creature,
            &mut state,
            &definition(),
            now,
        );
        assert_eq!(rejected, Err(OwnerTimerError::PendingLimitReached));
        // The rejection never advanced the bounded sequence a second time (binding item (a)).
        assert_eq!(state.next_sequence(), 1);

        let due = SemanticTimeMicros::from_micros(definition().think_interval_micros);
        let clock = VirtualOwnerClock::new(due);
        let fired = lane.drain_due(&clock, &owner_fence, |_| true);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].occurrence, first);

        let ordinal_3 = owner_fence
            .accept_input(generation)
            .expect("issue a live ordinal");
        let stamp_3 = owner_fence.stamp(ordinal_3);
        let second = schedule_next_think(
            &mut lane,
            &owner_fence,
            stamp_3,
            creature,
            &mut state,
            &definition(),
            due,
        )
        .expect("schedule succeeds after the first drained");
        assert_eq!(second.sequence, 1);
        assert_ne!(second, first, "an occurrence identity is never reused");
    }

    #[test]
    fn bite_readiness_respects_the_interval_and_starts_ready() {
        let definition = definition();
        let mut state = RatThinkState::new(pos(0, 0));
        let now = SemanticTimeMicros::from_micros(0);
        assert!(
            state.bite_ready(&definition, now),
            "no bite has ever committed"
        );
        state.record_bite_committed(now);
        assert!(!state.bite_ready(
            &definition,
            SemanticTimeMicros::from_micros(definition.bite_interval_micros - 1)
        ));
        assert!(state.bite_ready(
            &definition,
            SemanticTimeMicros::from_micros(definition.bite_interval_micros)
        ));
    }

    #[test]
    fn occurrence_decision_id_differs_by_sequence_and_creature() {
        let creature = actor(40);
        let id_a = occurrence_decision_id(ThinkOccurrence {
            creature,
            sequence: 0,
        });
        let id_b = occurrence_decision_id(ThinkOccurrence {
            creature,
            sequence: 1,
        });
        assert_ne!(id_a, id_b);
        let other_creature = occurrence_decision_id(ThinkOccurrence {
            creature: actor(41),
            sequence: 0,
        });
        assert_ne!(id_a, other_creature);
    }
}
