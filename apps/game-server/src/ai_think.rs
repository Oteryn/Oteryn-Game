//! AI-3 (`#162` §5; `GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1` §4.4): the
//! composition root `ai/mod.rs`'s own doc used to defer ("the composition root may elect to
//! include it only after coordinator integration"). It builds one creature's per-think decision
//! over an owner snapshot and, for chase/wander, acts through the real Movement owner
//! (`crate::movement`). Attack is decided here (adjacency, cooldown, the bite-chance draw) but
//! never executed: a successful draw is only surfaced as `ThinkOutcome::AttackIntent` for AI-4's
//! Ability boundary (§4.6) to issue later.
//!
//! This lives as a sibling of `ai/`, not inside it: `tests/ai_bootstrap.rs` path-includes
//! `ai/mod.rs` as its own standalone crate root (`#[path = "../src/ai/mod.rs"] mod ai;`), which
//! has no `foundation`/`content`/`movement`/`oteryn_simulation_determinism` available. Composing
//! against those from inside `ai/`'s own module tree would break that standalone build; keeping
//! the composition next to `ai/` instead (the task's own alternative -- "in `ai/` or a small
//! module next to it") avoids it while still depending only on `ai/`'s public surface, exactly as
//! any other caller of `ai::` would.
//!
//! Reuse of the existing `ai/` primitives:
//! - `ai::canonicalize_perception` selects the think's target: nearest perceived player wins,
//!   ties break by stable identity (§8), exactly the bootstrap's existing ordering. Added one
//!   accessor, `ai::CandidateId::get`, so this module can map a canonicalized candidate back to
//!   the caller-supplied `PerceivedPlayerId` it was built from.
//! - `ai::build_path_proposal` packages and bounds the chase tier's single greedy cardinal step
//!   as a real, resource-checked `PathProposal` (`RouteSteps`/`RouteBytes`/
//!   `PathRequestsPerActor`/`PathSearchWork` all stay enforced). Production pathfinding is out
//!   of this decision's scope (§7): the "path" is always exactly one step.
//! - `ai::AiProvenance` is threaded through unchanged as the path request's provenance.
//! - `ai::{Decision, DecisionUnit, resolve}` is deliberately left unmodified: it is the
//!   bootstrap's generic `Idle`/`AcquireCandidate` mechanism, authored externally by
//!   `DecisionUnit`s, and does not express attack/chase/wander tiers or randomness. Forcing
//!   `decide` below through that shape would not reduce risk, only readability, so the real §4.4
//!   policy lives here instead, exactly where `ai/mod.rs`'s own doc says the composition root
//!   belongs.
//! - `ai::AiSnapshot`/`ActorId` are also left unmodified: `AiSnapshot`'s `active_actors` models
//!   the owner's whole active-actor scope (bound by `AI01-ACTIVE-ACTORS`, 256), a different
//!   population from "players perceived within D115's 7 tiles" (bound by the bootstrap's own
//!   `PerceptionCandidates`, 64, already enforced inside `canonicalize_perception`). Reusing
//!   `AiSnapshot` for perceived players would be a semantic mismatch, not genuine reuse.
//!
//! Randomness (§4.4: "wander direction, wander chance, bite chance ... a SIM RNG stream bound to
//! the think occurrence. A retry of the same occurrence never redraws") is
//! `oteryn_simulation_determinism::deterministic_decision_u64`, purpose-isolated per draw and a
//! pure function of `(decision_root, occurrence, purpose, draw_index)`: calling it again for the
//! same occurrence always reproduces the same draw, satisfying "never redraws" by construction,
//! with no wall clock anywhere in this module.
//!
//! Timer binding (AI-1 `owner_timer.rs`, `#162` binding items (a)/(b), carried forward the same
//! way AI-2 applied them to respawn occurrences):
//! - **(a) monotonic per-actor occurrence identity.** `ThinkSequenceTracker` is the only place a
//!   `ThinkOccurrence`'s sequence is derived from: it advances monotonically per actor and never
//!   re-issues a value, so `(actor, sequence)` can never repeat across the tracker's lifetime.
//!   `OwnerTimerLane::schedule` independently refuses a duplicate `(family, occurrence)` pair as
//!   a second guarantee. `ThinkSequenceTracker::retire` bounds the tracker's size across
//!   respawns: the owner calls it on an actor's death/despawn, so the table holds an entry per
//!   currently live/pending actor, never one per past generation forever (Codex P2 on `#1196`).
//! - **(b) unforgeable stamp provenance.** `schedule_next_think` fabricates no
//!   `RuntimeWorkStamp`: it only forwards one the caller's `&ScopeRuntimeFence` already issued,
//!   and `OwnerTimerLane::schedule` itself is what checks `accepts_stamp`.
//!
//! Gap, reported per instruction rather than worked around: `foundation::owner_timer` was
//! declared `mod owner_timer;` (private, no re-export) with the comment "AI-2/AI-3 wire it into
//! `ChannelRuntimeV1`'s owner cycle from their own owned paths", but nothing outside
//! `foundation` could reach `OwnerTimerLane` at all before this task (see the
//! `foundation/mod.rs` visibility fix landed alongside this file). `ScopeRuntimeFence` itself has
//! no accessible constructor outside `foundation` (`from_external_grant`/`with_scope` are
//! private to that module, `#[allow(dead_code)]`, "production wiring is AI-2's `ChannelRuntimeV1`
//! integration" -- which did not happen: AI-2's own record leaves the lane's live caller for
//! later). So `schedule_next_think` below is real, compiled and type-checked against
//! `OwnerTimerLane::schedule`'s exact signature, but this task cannot itself construct a valid
//! fence to exercise it end-to-end in a test; that needs a `foundation`-owned follow-up (a
//! `ChannelRuntimeV1` accessor), outside this task's owned paths.

use crate::ai::{
    AiError, AiProvenance, Candidate, CandidateId, PathRequest, RouteStep, build_path_proposal,
    canonicalize_perception,
};
use crate::content::static_cell_engine::EngineeringStaticCellIndex;
use crate::foundation::owner_timer::{
    AI01_PENDING_TIMERS_PER_ACTOR, OwnerTimerError, OwnerTimerLane, SemanticTimeMicros, TimerFamily,
};
use crate::foundation::{
    CurrentOwnerMovementPosition, ExactActorRef, MovementLocalPosition, MovementPositionSnapshot,
    RuntimeWorkStamp, ScopeRuntimeFence,
};
use crate::movement::{CardinalStep, MovementEngineeringSelection, MovementError, step_cardinal};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

/// D115 (`#162` issuecomment-5879404970): the owner's fixed §4.4 behaviour constants for the
/// first creature slice. Bite interval/chance/magnitude are content inputs (§4.8), not D115
/// values, and are never hardcoded here: they arrive per-think as `AttackReadiness`.
pub const D115_THINK_INTERVAL_MILLIS: u64 = 1_000;
pub const D115_PERCEPTION_RANGE_TILES: i64 = 7;
pub const D115_WANDER_CHANCE_PERCENT: u8 = 25;
pub const D115_WANDER_RADIUS_TILES: i64 = 2;

const PURPOSE_ATTACK_CHANCE: &str = "ai.think.attack_chance";
const PURPOSE_WANDER_CHANCE: &str = "ai.think.wander_chance";
const PURPOSE_WANDER_DIRECTION: &str = "ai.think.wander_direction";

const CARDINAL_STEP_ENCODED_BYTES: usize = 1;

/// A caller-supplied stable identity for a perceived player. `ExactActorRef`'s fields and
/// constructor are private to `foundation` by design ("never decoded from a client handle"), so
/// this module cannot derive a numeric identity from one itself; the caller (the future
/// production owner-snapshot builder, or a test) supplies one however it has a persistent player
/// identity available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PerceivedPlayerId(u64);

impl PerceivedPlayerId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// §4.6's future Ability boundary and §4.7's HP floor are not wired yet (AI-4, spell P3b-2), so
/// bite cooldown/chance arrive as caller-supplied facts rather than being read from a live
/// vitals/cooldown store that does not exist yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackReadiness {
    pub off_cooldown: bool,
    pub chance_percent: u8,
}

/// §4.4's re-entry protection note ("a player inside the active four-second PvE re-entry
/// protection window ... is not a legal attack target. The creature may still perceive and
/// chase that player") arrives as `legal_attack_target`: the composition root's own facts
/// arrival, since no protection-window fact source is wired yet either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerceivedPlayer {
    pub id: PerceivedPlayerId,
    pub position: MovementLocalPosition,
    pub legal_attack_target: bool,
}

/// One creature's owner snapshot for one think (§4.4). `decision_root`/`occurrence` bind the
/// think's randomness to this exact think occurrence; `path_work_id` is the chase tier's
/// `PathRequest::work_id`.
#[derive(Clone)]
pub struct CreatureThinkInput {
    pub provenance: AiProvenance,
    pub decision_root: GameplayDecisionRoot,
    pub occurrence: DecisionOccurrenceId,
    pub path_work_id: u64,
    pub position: MovementLocalPosition,
    pub home: MovementLocalPosition,
    pub attack: AttackReadiness,
}

/// §4.4's per-think resolution: at most one action is proposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkOutcome {
    /// A successful bite-chance draw against an adjacent, legal, off-cooldown target. Never
    /// executed by this module (AI-4 issues it through Ability).
    AttackIntent(PerceivedPlayerId),
    ChaseStep(CardinalStep),
    WanderStep(CardinalStep),
    Idle,
}

fn chebyshev_distance(from: MovementLocalPosition, to: MovementLocalPosition) -> Option<i64> {
    if from.floor != to.floor {
        return None;
    }
    let dx = i64::from(to.x) - i64::from(from.x);
    let dy = i64::from(to.y) - i64::from(from.y);
    Some(dx.abs().max(dy.abs()))
}

const fn step_delta(step: CardinalStep) -> (i32, i32) {
    match step {
        CardinalStep::North => (0, -1),
        CardinalStep::East => (1, 0),
        CardinalStep::South => (0, 1),
        CardinalStep::West => (-1, 0),
    }
}

fn apply_step(position: MovementLocalPosition, step: CardinalStep) -> MovementLocalPosition {
    let (dx, dy) = step_delta(step);
    MovementLocalPosition {
        x: position.x.saturating_add(dx),
        y: position.y.saturating_add(dy),
        floor: position.floor,
    }
}

/// One greedy cardinal step reducing distance to `to` (§4.5: production pathfinding is out of
/// scope, §7; the "path" this decision proposes is always exactly one step). Ties prefer the
/// axis with the larger remaining delta, then the horizontal axis: a fixed, deterministic
/// tie-break, not a claim of an optimal or Reference route.
fn step_towards(from: MovementLocalPosition, to: MovementLocalPosition) -> Option<CardinalStep> {
    if from.floor != to.floor {
        return None;
    }
    let dx = i64::from(to.x) - i64::from(from.x);
    let dy = i64::from(to.y) - i64::from(from.y);
    if dx == 0 && dy == 0 {
        return None;
    }
    Some(if dx.abs() >= dy.abs() {
        if dx > 0 {
            CardinalStep::East
        } else {
            CardinalStep::West
        }
    } else if dy > 0 {
        CardinalStep::South
    } else {
        CardinalStep::North
    })
}

const fn cardinal_step_node(step: CardinalStep) -> u64 {
    match step {
        CardinalStep::North => 0,
        CardinalStep::East => 1,
        CardinalStep::South => 2,
        CardinalStep::West => 3,
    }
}

const fn decode_cardinal_step(node: u64) -> Option<CardinalStep> {
    match node {
        0 => Some(CardinalStep::North),
        1 => Some(CardinalStep::East),
        2 => Some(CardinalStep::South),
        3 => Some(CardinalStep::West),
        _ => None,
    }
}

fn roll_percent(
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    purpose: &str,
    chance_percent: u8,
) -> Result<bool, AiError> {
    let value = deterministic_decision_u64(root, occurrence, purpose, 0)
        .map_err(|_| AiError::InvalidInput)?;
    Ok(value % 100 < u64::from(chance_percent.min(100)))
}

fn pick_direction(
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
    purpose: &str,
    choices: &[CardinalStep],
) -> Result<Option<CardinalStep>, AiError> {
    if choices.is_empty() {
        return Ok(None);
    }
    let value = deterministic_decision_u64(root, occurrence, purpose, 0)
        .map_err(|_| AiError::InvalidInput)?;
    let choice_count = u64::try_from(choices.len()).map_err(|_| AiError::InvalidInput)?;
    let index = usize::try_from(value % choice_count).map_err(|_| AiError::InvalidInput)?;
    Ok(choices.get(index).copied())
}

/// `GAME-AI-01-ACTION-INTEGRATION-FIRST-CREATURE-SLICE-V1` §4.4: the deterministic, bounded
/// per-think resolution over one creature's owner snapshot and its perceived players.
pub fn decide(
    input: &CreatureThinkInput,
    perceived: &[PerceivedPlayer],
) -> Result<ThinkOutcome, AiError> {
    let candidates = perceived
        .iter()
        .filter_map(|player| {
            let distance = chebyshev_distance(input.position, player.position)?;
            if distance > D115_PERCEPTION_RANGE_TILES {
                return None;
            }
            let priority = u64::try_from(D115_PERCEPTION_RANGE_TILES - distance).ok()?;
            Some(Candidate::new(CandidateId::new(player.id.get()), priority))
        })
        .collect::<Vec<_>>();
    let perception = canonicalize_perception(&candidates)?;

    if let Some(nearest) = perception.candidates().first() {
        let target = perceived
            .iter()
            .find(|player| player.id.get() == nearest.id().get())
            .ok_or(AiError::InvalidInput)?;

        // 1. Attack (§4.4 tier 1): adjacent, legal and off cooldown. A failed draw ends this
        // think idle -- no chase or wander fallback, exactly per §4.4's wording.
        if target.legal_attack_target
            && input.attack.off_cooldown
            && chebyshev_distance(input.position, target.position) == Some(1)
        {
            return Ok(
                if roll_percent(
                    &input.decision_root,
                    input.occurrence,
                    PURPOSE_ATTACK_CHANCE,
                    input.attack.chance_percent,
                )? {
                    ThinkOutcome::AttackIntent(target.id)
                } else {
                    ThinkOutcome::Idle
                },
            );
        }

        // 2. Chase (§4.4 tier 2 / §4.5): one cardinal step, packaged and bounded as a real
        // `PathProposal`.
        if let Some(step) = step_towards(input.position, target.position) {
            let route = [RouteStep::new(
                cardinal_step_node(step),
                CARDINAL_STEP_ENCODED_BYTES,
            )];
            let proposal = build_path_proposal(
                PathRequest::new(input.provenance, input.path_work_id, 1, 1),
                &route,
            )?;
            let proposed_step = proposal
                .steps()
                .first()
                .and_then(|route_step| decode_cardinal_step(route_step.node()))
                .ok_or(AiError::InvalidInput)?;
            return Ok(ThinkOutcome::ChaseStep(proposed_step));
        }
    }

    // 3. Wander (§4.4 tier 3 / D115): a fixed chance of one random cardinal step that stays
    // within the wander radius of the spawn's home cell; otherwise idle.
    if roll_percent(
        &input.decision_root,
        input.occurrence,
        PURPOSE_WANDER_CHANCE,
        D115_WANDER_CHANCE_PERCENT,
    )? {
        let valid_steps: Vec<CardinalStep> = [
            CardinalStep::North,
            CardinalStep::East,
            CardinalStep::South,
            CardinalStep::West,
        ]
        .into_iter()
        .filter(|step| {
            chebyshev_distance(apply_step(input.position, *step), input.home)
                .is_some_and(|distance| distance <= D115_WANDER_RADIUS_TILES)
        })
        .collect();
        if let Some(step) = pick_direction(
            &input.decision_root,
            input.occurrence,
            PURPOSE_WANDER_DIRECTION,
            &valid_steps,
        )? {
            return Ok(ThinkOutcome::WanderStep(step));
        }
    }
    Ok(ThinkOutcome::Idle)
}

/// What actually happened after `decide`'s outcome was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkAction {
    /// Surfaced only; AI-4 issues it through Ability. Out of this module's mutation surface.
    AttackIntentSurfaced(PerceivedPlayerId),
    Moved(MovementLocalPosition),
    Idle,
}

/// §4.4/§4.5: chase and wander act through the real Movement owner (`crate::movement::
/// step_cardinal`, the same revalidating/committing path a player's step takes); attack is only
/// surfaced. A rejected movement proposal (blocked, stale snapshot, scope mismatch, ...) mutates
/// nothing (`step_cardinal`'s own contract) and is returned as an error for the caller to log and
/// proceed to scheduling the next think, per §4.4: "the owner rejects every proposal it cannot
/// legally apply; a rejection changes nothing except scheduling the next think."
pub fn act_step(
    outcome: ThinkOutcome,
    owner: &mut CurrentOwnerMovementPosition<'_>,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    selection: &MovementEngineeringSelection<'_>,
    index: &(impl crate::content::native_cell_lookup::NativeStaticCellLookup + ?Sized),
) -> Result<ThinkAction, MovementError> {
    match outcome {
        ThinkOutcome::AttackIntent(target) => Ok(ThinkAction::AttackIntentSurfaced(target)),
        ThinkOutcome::Idle => Ok(ThinkAction::Idle),
        ThinkOutcome::ChaseStep(step) | ThinkOutcome::WanderStep(step) => {
            let snapshot = step_cardinal(owner, actor, expected, selection, index, step)?;
            Ok(ThinkAction::Moved(snapshot.position()))
        }
    }
}

/// §4.2: the AI think timer family. Its registered ceiling is AI-1's
/// `AI01_PENDING_TIMERS_PER_ACTOR` (`RESOURCE_LIMITS_REGISTRY.json`, AI-RL-06); this is the
/// first concrete `TimerFamily` checked against it outside `owner_timer`'s own tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThinkFamily;

impl TimerFamily for ThinkFamily {
    fn registered_maximum(self) -> usize {
        AI01_PENDING_TIMERS_PER_ACTOR
    }
}

/// §4.2: "a think occurrence is (creature `ExactActorRef`, per-actor think sequence number)."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThinkOccurrence {
    pub actor: ExactActorRef,
    pub sequence: u64,
}

/// Binding item (a): the only place a `ThinkOccurrence`'s sequence is derived from. It advances
/// monotonically per actor and never re-issues a value, so `(actor, sequence)` can never repeat
/// across this tracker's lifetime. `ExactActorRef` derives no `Hash` (its fields are private to
/// `foundation`), so this is a small linear table rather than a hash map -- bounded by the D57
/// envelope (64 live creatures), not a concern at that size.
///
/// Bounded across respawns (Codex P2 on `#1196`): each respawn's new actor-local generation is a
/// distinct `ExactActorRef` (`foundation`'s own doc: "never decoded from a client handle", so
/// this module cannot itself compare two refs' generations to tell "the same slot, a newer
/// generation" from "an unrelated actor" -- `ExactActorRef`'s fields are private to
/// `foundation`). Without `retire`, a dead generation's entry would never be removed and the
/// table would grow one stale entry per past generation forever. `retire` is the bounded
/// alternative: the owner (which does hold generation identity) calls it on that actor's death
/// or despawn, so the entry count tracks currently live/pending actors, never total history.
#[derive(Debug, Default)]
pub struct ThinkSequenceTracker {
    next: Vec<(ExactActorRef, u64)>,
}

impl ThinkSequenceTracker {
    #[must_use]
    pub const fn new() -> Self {
        Self { next: Vec::new() }
    }

    pub fn next_occurrence(&mut self, actor: ExactActorRef) -> ThinkOccurrence {
        if let Some(entry) = self
            .next
            .iter_mut()
            .find(|(candidate, _)| *candidate == actor)
        {
            let sequence = entry.1;
            entry.1 = entry.1.saturating_add(1);
            ThinkOccurrence { actor, sequence }
        } else {
            self.next.push((actor, 1));
            ThinkOccurrence { actor, sequence: 0 }
        }
    }

    /// Removes `actor`'s entry, if any. The owner calls this exactly once per actor death or
    /// despawn (before or independent of a respawn's fresh `ExactActorRef`), which is what
    /// bounds this tracker's size to currently live/pending actors. A safe no-op (returns
    /// `false`, changes nothing) for an actor that was never scheduled or was already retired.
    pub fn retire(&mut self, actor: ExactActorRef) -> bool {
        let before = self.next.len();
        self.next.retain(|(candidate, _)| *candidate != actor);
        self.next.len() != before
    }

    /// The number of actors this tracker currently holds a sequence for. Bounded by the D57
    /// envelope (64) as long as the owner retires every dead/despawned actor.
    #[must_use]
    pub fn len(&self) -> usize {
        self.next.len()
    }

    /// Whether this tracker currently holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.next.is_empty()
    }
}

/// Binding item (b): inherited from `OwnerTimerLane::schedule` itself, never re-implemented
/// here. `scheduling_stamp` must already have been issued by `owner_fence` for the current
/// resolution; this wrapper fabricates no stamp of its own and forwards exactly what the
/// caller's fence vouches for, so `schedule`'s own `accepts_stamp` check is the sole enforcement
/// point. See the module doc for why this cannot be exercised end-to-end from this task's own
/// tests (`ScopeRuntimeFence` has no accessible constructor outside `foundation`).
pub fn schedule_next_think(
    lane: &mut OwnerTimerLane<ThinkFamily, ThinkOccurrence>,
    owner_fence: &ScopeRuntimeFence,
    scheduling_stamp: RuntimeWorkStamp,
    tracker: &mut ThinkSequenceTracker,
    actor: ExactActorRef,
    due: SemanticTimeMicros,
) -> Result<ThinkOccurrence, OwnerTimerError> {
    let occurrence = tracker.next_occurrence(actor);
    lane.schedule(
        owner_fence,
        scheduling_stamp,
        ThinkFamily,
        occurrence,
        Some(actor),
        due,
    )?;
    Ok(occurrence)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{AiProvenanceInput, ResourceLimit};
    use crate::content::static_cell_engine::{
        EngineeringCollisionClaim, EngineeringStaticCellClaim, EngineeringStaticCellScope,
    };
    use crate::content::{
        CollisionClass, ContentLockBinding, ContentLockEntry, CoordinateFrameRef, LogicalCell,
        MapRevisionRef, ProductionAtom, ProductionKey, Sha256HexDigest,
    };
    use crate::foundation::owner_timer::{CatchUpPolicy, FamilyPolicy};
    use crate::foundation::{MovementActorFixture, RuntimeScopeRefV1, ScopeOwnershipGeneration};
    use std::error::Error;

    fn provenance() -> AiProvenance {
        AiProvenance::new(AiProvenanceInput {
            scope_id: 1,
            scope_generation: 2,
            actor_generation: 3,
            behavior_revision: 4,
            content_revision: 5,
            navigation_revision: 6,
            ruleset_revision: 7,
            determinism_profile_revision: 8,
        })
    }

    fn root(seed: u8) -> GameplayDecisionRoot {
        GameplayDecisionRoot::from_bytes([seed; 32])
    }

    fn occurrence(seed: u8) -> DecisionOccurrenceId {
        DecisionOccurrenceId::from_bytes([seed; 16])
    }

    fn position(x: i32, y: i32) -> MovementLocalPosition {
        MovementLocalPosition { x, y, floor: 7 }
    }

    fn input(
        seed: u8,
        position: MovementLocalPosition,
        home: MovementLocalPosition,
    ) -> CreatureThinkInput {
        CreatureThinkInput {
            provenance: provenance(),
            decision_root: root(seed),
            occurrence: occurrence(seed),
            path_work_id: 1,
            position,
            home,
            attack: AttackReadiness {
                off_cooldown: true,
                chance_percent: 0,
            },
        }
    }

    #[test]
    fn d115_constants_are_pinned() {
        assert_eq!(D115_THINK_INTERVAL_MILLIS, 1_000);
        assert_eq!(D115_PERCEPTION_RANGE_TILES, 7);
        assert_eq!(D115_WANDER_CHANCE_PERCENT, 25);
        assert_eq!(D115_WANDER_RADIUS_TILES, 2);
    }

    #[test]
    fn no_perceived_player_and_no_wander_draw_stays_idle() -> Result<(), AiError> {
        // A decision root/occurrence combination whose wander draw fails (chance 25%, so most
        // seeds miss); assert against the real draw instead of assuming a fixed seed always
        // misses.
        let home = position(0, 0);
        let mut found_idle = false;
        for seed in 0_u8..8 {
            let think_input = input(seed, home, home);
            if decide(&think_input, &[])? == ThinkOutcome::Idle {
                found_idle = true;
                break;
            }
        }
        assert!(
            found_idle,
            "expected at least one idle wander miss in 8 seeds"
        );
        Ok(())
    }

    #[test]
    fn wander_step_stays_within_radius_and_is_reachable() -> Result<(), AiError> {
        let home = position(10, 10);
        let mut found_wander = false;
        for seed in 0_u8..32 {
            let think_input = input(seed, home, home);
            if let ThinkOutcome::WanderStep(step) = decide(&think_input, &[])? {
                found_wander = true;
                let destination = apply_step(home, step);
                assert!(
                    chebyshev_distance(destination, home)
                        .is_some_and(|distance| distance <= D115_WANDER_RADIUS_TILES)
                );
            }
        }
        assert!(
            found_wander,
            "expected at least one wander step in 32 seeds"
        );
        Ok(())
    }

    #[test]
    fn decide_is_deterministic_for_the_same_occurrence() -> Result<(), AiError> {
        let home = position(0, 0);
        let think_input = input(3, home, home);
        let first = decide(&think_input, &[])?;
        let second = decide(&think_input, &[])?;
        assert_eq!(
            first, second,
            "a retry of the same occurrence never redraws"
        );
        Ok(())
    }

    #[test]
    fn nearest_perceived_player_wins_and_far_players_are_excluded() -> Result<(), AiError> {
        let self_position = position(0, 0);
        let near = PerceivedPlayer {
            id: PerceivedPlayerId::new(1),
            position: position(3, 0),
            legal_attack_target: true,
        };
        let far_in_range = PerceivedPlayer {
            id: PerceivedPlayerId::new(2),
            position: position(6, 0),
            legal_attack_target: true,
        };
        let out_of_range = PerceivedPlayer {
            id: PerceivedPlayerId::new(3),
            position: position(8, 0),
            legal_attack_target: true,
        };
        let think_input = input(1, self_position, self_position);
        let outcome = decide(&think_input, &[near, far_in_range, out_of_range])?;
        // Nearest (`near`, distance 3) is not adjacent, so tier 2 chases it: East reduces the
        // x-distance toward x=3.
        assert_eq!(outcome, ThinkOutcome::ChaseStep(CardinalStep::East));
        Ok(())
    }

    #[test]
    fn re_entry_protected_adjacent_target_is_chased_not_attacked() -> Result<(), AiError> {
        let self_position = position(0, 0);
        let protected = PerceivedPlayer {
            id: PerceivedPlayerId::new(1),
            position: position(1, 0),
            legal_attack_target: false,
        };
        let mut think_input = input(1, self_position, self_position);
        think_input.attack.chance_percent = 100;
        let outcome = decide(&think_input, &[protected])?;
        assert_eq!(outcome, ThinkOutcome::ChaseStep(CardinalStep::East));
        Ok(())
    }

    #[test]
    fn adjacent_legal_off_cooldown_target_can_draw_an_attack_intent() -> Result<(), AiError> {
        let self_position = position(0, 0);
        let target = PerceivedPlayer {
            id: PerceivedPlayerId::new(9),
            position: position(1, 0),
            legal_attack_target: true,
        };
        let mut think_input = input(1, self_position, self_position);
        think_input.attack.chance_percent = 100;
        let outcome = decide(&think_input, &[target])?;
        assert_eq!(
            outcome,
            ThinkOutcome::AttackIntent(PerceivedPlayerId::new(9))
        );
        Ok(())
    }

    #[test]
    fn a_failed_attack_draw_ends_the_think_idle_with_no_chase_or_wander_fallback()
    -> Result<(), AiError> {
        let self_position = position(0, 0);
        let target = PerceivedPlayer {
            id: PerceivedPlayerId::new(9),
            position: position(1, 0),
            legal_attack_target: true,
        };
        let mut think_input = input(1, self_position, self_position);
        think_input.attack.chance_percent = 0;
        let outcome = decide(&think_input, &[target])?;
        assert_eq!(outcome, ThinkOutcome::Idle);
        Ok(())
    }

    #[test]
    fn cooling_down_adjacent_target_falls_through_to_chase() -> Result<(), AiError> {
        let self_position = position(0, 0);
        let target = PerceivedPlayer {
            id: PerceivedPlayerId::new(9),
            position: position(1, 0),
            legal_attack_target: true,
        };
        let mut think_input = input(1, self_position, self_position);
        think_input.attack.off_cooldown = false;
        think_input.attack.chance_percent = 100;
        let outcome = decide(&think_input, &[target])?;
        assert_eq!(outcome, ThinkOutcome::ChaseStep(CardinalStep::East));
        Ok(())
    }

    #[test]
    fn too_many_perceived_players_fails_closed_through_the_bootstrap_ceiling()
    -> Result<(), Box<dyn Error>> {
        let self_position = position(0, 0);
        let maximum = ResourceLimit::PerceptionCandidates.maximum();
        let over_limit: Vec<PerceivedPlayer> = (0..=maximum)
            .map(|value| {
                Ok(PerceivedPlayer {
                    id: PerceivedPlayerId::new(u64::try_from(value)?),
                    position: position(1, 0),
                    legal_attack_target: false,
                })
            })
            .collect::<Result<_, std::num::TryFromIntError>>()?;
        let think_input = input(1, self_position, self_position);
        assert_eq!(
            decide(&think_input, &over_limit),
            Err(AiError::CapacityExceeded(
                ResourceLimit::PerceptionCandidates
            ))
        );
        Ok(())
    }

    #[test]
    fn think_sequence_tracker_never_repeats_and_is_independent_per_actor() {
        let actor_a = ExactActorRef::transport_fixture(world_id(), channel_id(1));
        let actor_b = ExactActorRef::transport_fixture(world_id(), channel_id(2));
        let mut tracker = ThinkSequenceTracker::new();
        let first = tracker.next_occurrence(actor_a);
        let second = tracker.next_occurrence(actor_a);
        let third = tracker.next_occurrence(actor_b);
        assert_eq!(first.sequence, 0);
        assert_eq!(second.sequence, 1);
        assert_ne!(
            (first.actor, first.sequence),
            (second.actor, second.sequence)
        );
        assert_eq!(third.sequence, 0);
        assert_eq!(third.actor, actor_b);
    }

    #[test]
    fn think_sequence_tracker_sequence_is_monotonic_within_one_generation() {
        let actor = ExactActorRef::transport_fixture(world_id(), channel_id(1));
        let mut tracker = ThinkSequenceTracker::new();
        let first = tracker.next_occurrence(actor);
        let second = tracker.next_occurrence(actor);
        let third = tracker.next_occurrence(actor);
        assert_eq!([first.sequence, second.sequence, third.sequence], [0, 1, 2]);
        assert_eq!(
            tracker.len(),
            1,
            "still one entry: the same live generation"
        );
    }

    #[test]
    fn think_sequence_tracker_stays_bounded_across_many_respawn_generations_of_one_slot() {
        // Codex P2 on #1196: without `retire`, this tracker would grow one stale entry per past
        // generation forever. Each iteration here stands in for one respawn's fresh
        // `ExactActorRef` (a distinct generation is, by construction, a distinct ref) of what is
        // conceptually the same spawn slot: schedule its think, then retire it on death, exactly
        // as the owner would across many respawn cycles of one cell.
        let mut tracker = ThinkSequenceTracker::new();
        for generation_seed in 1_u8..=200 {
            let actor = ExactActorRef::transport_fixture(world_id(), channel_id(generation_seed));
            let occurrence = tracker.next_occurrence(actor);
            assert_eq!(
                occurrence.sequence, 0,
                "a fresh generation starts a new sequence"
            );
            assert_eq!(
                tracker.len(),
                1,
                "at most one entry while this generation is live"
            );
            assert!(
                tracker.retire(actor),
                "retire removes the dead generation's entry"
            );
        }
        assert!(
            tracker.is_empty(),
            "no stale entries survive 200 respawn generations"
        );
    }

    #[test]
    fn think_sequence_tracker_retire_is_a_safe_no_op_for_an_untracked_or_already_retired_actor() {
        let tracked = ExactActorRef::transport_fixture(world_id(), channel_id(1));
        let untracked = ExactActorRef::transport_fixture(world_id(), channel_id(2));
        let mut tracker = ThinkSequenceTracker::new();
        assert!(!tracker.retire(untracked), "nothing was ever scheduled");

        tracker.next_occurrence(tracked);
        assert!(tracker.retire(tracked));
        assert!(
            !tracker.retire(tracked),
            "retiring an already-retired actor changes nothing"
        );
        assert!(tracker.is_empty());
    }

    #[test]
    fn think_family_registered_maximum_matches_ai01_pending_timers_per_actor() {
        assert_eq!(
            ThinkFamily.registered_maximum(),
            AI01_PENDING_TIMERS_PER_ACTOR
        );
    }

    #[test]
    fn think_family_lane_construction_admits_the_maximum_and_rejects_maximum_plus_one() {
        let scope = RuntimeScopeRefV1::channel(world_id(), channel_id(9));
        let generation = generation(1);
        let at_maximum = OwnerTimerLane::<ThinkFamily, ThinkOccurrence>::for_generation(
            scope,
            generation,
            [(
                ThinkFamily,
                FamilyPolicy {
                    max_pending: AI01_PENDING_TIMERS_PER_ACTOR,
                    catch_up: CatchUpPolicy::SkipToLatest,
                },
            )],
        );
        assert!(at_maximum.is_ok());

        let over_maximum = OwnerTimerLane::<ThinkFamily, ThinkOccurrence>::for_generation(
            scope,
            generation,
            [(
                ThinkFamily,
                FamilyPolicy {
                    max_pending: AI01_PENDING_TIMERS_PER_ACTOR + 1,
                    catch_up: CatchUpPolicy::SkipToLatest,
                },
            )],
        );
        assert_eq!(
            over_maximum.err(),
            Some(OwnerTimerError::FamilyCapExceedsRegisteredMaximum)
        );
    }

    fn generation(value: u64) -> ScopeOwnershipGeneration {
        match ScopeOwnershipGeneration::new(value) {
            Ok(generation) => generation,
            Err(_) => unreachable!("test-fixed nonzero generation"),
        }
    }

    fn uuid_v7(seed: u8) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        bytes[0] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = seed;
        bytes
    }

    fn world_id() -> crate::foundation::WorldId {
        match crate::foundation::WorldId::decode(&uuid_v7(1)) {
            Ok(id) => id,
            Err(_) => unreachable!("test-fixed UUIDv7 WorldId"),
        }
    }

    fn channel_id(seed: u8) -> crate::foundation::ChannelId {
        match crate::foundation::ChannelId::decode(&uuid_v7(seed)) {
            Ok(id) => id,
            Err(_) => unreachable!("test-fixed UUIDv7 ChannelId"),
        }
    }

    fn cell_scope(
        for_world: crate::foundation::WorldId,
    ) -> Result<EngineeringStaticCellScope, Box<dyn Error>> {
        Ok(EngineeringStaticCellScope {
            world_id: for_world,
            coordinate_frame: CoordinateFrameRef::new("ai-think-engineering-frame")?,
            map_revision: MapRevisionRef::new("ai-think-engineering-map")?,
            generation_digest: [9; 32],
            content_lock: ContentLockBinding {
                revision_digest_token: ProductionAtom::new("lock", "ai-think-engineering-lock")?,
                entries: vec![ContentLockEntry::exact(
                    ProductionKey::new("engineering:ai-think")?,
                    ProductionAtom::new("revision", "ai-think-engineering-r1")?,
                    Sha256HexDigest::new(&"b".repeat(64))?,
                )],
            },
        })
    }

    fn walkable_index(
        scope: &EngineeringStaticCellScope,
        cells: &[(i32, i32, i32)],
    ) -> Result<EngineeringStaticCellIndex, Box<dyn Error>> {
        Ok(EngineeringStaticCellIndex::from_claims(
            cells
                .iter()
                .map(|(x, y, z)| EngineeringStaticCellClaim {
                    scope: scope.clone(),
                    cell: LogicalCell {
                        x: *x,
                        y: *y,
                        z: *z,
                    },
                    collision: EngineeringCollisionClaim::Qualified(CollisionClass::Walkable),
                })
                .collect(),
        )?)
    }

    #[test]
    fn act_step_chase_moves_a_creature_actor_through_the_real_movement_owner()
    -> Result<(), Box<dyn Error>> {
        // §4.5: creature slots are Movement-capable through the same owner turn a player's step
        // uses; this exercises exactly that path for AI-3's own composed step.
        let mut creature = MovementActorFixture::new(position(4, 5), true)?;
        let scope = cell_scope(creature.world_id())?;
        let index = walkable_index(&scope, &[(5, 5, 7)])?;
        let actor = creature.actor();
        let before = creature.current_position()?;
        let selection = MovementEngineeringSelection {
            owner_context: before.context(),
            content_scope: &scope,
        };
        let outcome = act_step(
            ThinkOutcome::ChaseStep(CardinalStep::East),
            &mut creature.borrow_position(),
            actor,
            before,
            &selection,
            &index,
        )?;
        assert_eq!(outcome, ThinkAction::Moved(position(5, 5)));
        assert_eq!(creature.current_position()?.position(), position(5, 5));
        Ok(())
    }

    #[test]
    fn act_step_wander_moves_a_creature_actor_through_the_real_movement_owner()
    -> Result<(), Box<dyn Error>> {
        let mut creature = MovementActorFixture::new(position(4, 5), true)?;
        let scope = cell_scope(creature.world_id())?;
        let index = walkable_index(&scope, &[(4, 4, 7)])?;
        let actor = creature.actor();
        let before = creature.current_position()?;
        let selection = MovementEngineeringSelection {
            owner_context: before.context(),
            content_scope: &scope,
        };
        let outcome = act_step(
            ThinkOutcome::WanderStep(CardinalStep::North),
            &mut creature.borrow_position(),
            actor,
            before,
            &selection,
            &index,
        )?;
        assert_eq!(outcome, ThinkAction::Moved(position(4, 4)));
        Ok(())
    }

    #[test]
    fn act_step_never_moves_for_attack_intent_or_idle() -> Result<(), Box<dyn Error>> {
        let mut creature = MovementActorFixture::new(position(4, 5), true)?;
        let scope = cell_scope(creature.world_id())?;
        let index = walkable_index(&scope, &[(5, 5, 7)])?;
        let actor = creature.actor();
        let before = creature.current_position()?;
        let selection = MovementEngineeringSelection {
            owner_context: before.context(),
            content_scope: &scope,
        };
        let idle = act_step(
            ThinkOutcome::Idle,
            &mut creature.borrow_position(),
            actor,
            before,
            &selection,
            &index,
        )?;
        assert_eq!(idle, ThinkAction::Idle);
        let attack = act_step(
            ThinkOutcome::AttackIntent(PerceivedPlayerId::new(1)),
            &mut creature.borrow_position(),
            actor,
            before,
            &selection,
            &index,
        )?;
        assert_eq!(
            attack,
            ThinkAction::AttackIntentSurfaced(PerceivedPlayerId::new(1))
        );
        assert_eq!(creature.current_position()?, before, "no movement mutation");
        Ok(())
    }

    #[test]
    fn act_step_propagates_a_blocked_movement_error_without_mutating() -> Result<(), Box<dyn Error>>
    {
        let mut creature = MovementActorFixture::new(position(4, 5), true)?;
        let scope = cell_scope(creature.world_id())?;
        let index = EngineeringStaticCellIndex::from_claims(vec![EngineeringStaticCellClaim {
            scope: scope.clone(),
            cell: LogicalCell { x: 5, y: 5, z: 7 },
            collision: EngineeringCollisionClaim::Qualified(CollisionClass::Blocked),
        }])?;
        let actor = creature.actor();
        let before = creature.current_position()?;
        let selection = MovementEngineeringSelection {
            owner_context: before.context(),
            content_scope: &scope,
        };
        let result = act_step(
            ThinkOutcome::ChaseStep(CardinalStep::East),
            &mut creature.borrow_position(),
            actor,
            before,
            &selection,
            &index,
        );
        assert_eq!(result, Err(MovementError::Blocked));
        assert_eq!(creature.current_position()?, before);
        Ok(())
    }
}
