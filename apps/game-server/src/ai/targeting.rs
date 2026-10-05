//! CREATURE-AI-1 §2.1 items 2-4: perception, eligibility and target selection.
//!
//! Perception is VIS-1's interest predicate (`movement/interest.rs` `can_see`) seen from the
//! player: a creature perceives a player whose interest area contains it. There is no second
//! geometry. Candidates are ordered by MOVE-RL-11's canonical key (floor distance, plane
//! Chebyshev distance, identity) and bounded by `AI01-PERCEPTION-CANDIDATES` (64), nearest first
//! beyond it. Selection is a pure function of the candidates and the caller's draws; the think
//! (`ai_think.rs`) owns the draws, the order of selection steps and the target slot.
//!
//! Declared from `ai_think.rs` with `#[path]` (see `behaviour_profile.rs`).

use crate::ai::ResourceLimit;
use crate::content::{ProjectV2StrategyWeights, ProjectV2Targeting};
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use crate::movement::interest::{VisibilityPosition, VisibilitySettings};

/// One player the owner offers to a think. `eligible` folds the owner's facts of §2.1 item 3
/// (alive, attackable, not in a protection zone, no re-entry or login protection); `health` and
/// `damage` (the D132 contributor total this player dealt to the creature) feed the strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetCandidate {
    pub actor: ExactActorRef,
    pub position: MovementLocalPosition,
    pub eligible: bool,
    pub health: u64,
    pub damage: u64,
}

/// The four `strategy_weights` strategies, in the order the weight draw walks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    Nearest,
    LowestHealth,
    MostDamage,
    Random,
}

/// Whether a player at `player` sees a creature at `creature` under the reference view. A
/// source world's native floors can lie outside VIS-1's 0..=15; there a creature on the
/// player's own floor is judged on one plane and any other floor is not seen (the same mapping
/// as ATTACK-1b's `gameplay_transport::attack::sees`).
#[must_use]
pub fn sees(player: MovementLocalPosition, creature: MovementLocalPosition) -> bool {
    const PLANE: i16 = 7;
    let view = |position: MovementLocalPosition, floor: i16| {
        VisibilityPosition::new(position.x, position.y, floor).ok()
    };
    let (observer, seen) = match (view(player, player.floor), view(creature, creature.floor)) {
        (Some(observer), Some(seen)) => (Some(observer), Some(seen)),
        _ if player.floor == creature.floor => (view(player, PLANE), view(creature, PLANE)),
        _ => (None, None),
    };
    match (observer, seen) {
        (Some(observer), Some(seen)) => VisibilitySettings::REFERENCE.can_see(observer, seen),
        _ => false,
    }
}

/// Plane Chebyshev distance, ignoring floors.
#[must_use]
pub fn plane_distance(from: MovementLocalPosition, to: MovementLocalPosition) -> u64 {
    let dx = (i64::from(to.x) - i64::from(from.x)).unsigned_abs();
    let dy = (i64::from(to.y) - i64::from(from.y)).unsigned_abs();
    dx.max(dy)
}

/// MOVE-RL-11's canonical key from `origin`.
fn canonical_key(
    origin: MovementLocalPosition,
    candidate: &TargetCandidate,
) -> (u16, u64, [u8; 16]) {
    (
        origin.floor.abs_diff(candidate.position.floor),
        plane_distance(origin, candidate.position),
        candidate.actor.placement_identity(),
    )
}

/// §2.1 item 2: the players that perceive `creature`, canonically ordered, at most 64.
#[must_use]
pub fn perceive(
    creature: MovementLocalPosition,
    players: &[TargetCandidate],
) -> Vec<TargetCandidate> {
    let mut perceived = players
        .iter()
        .filter(|player| sees(player.position, creature))
        .copied()
        .collect::<Vec<_>>();
    perceived.sort_by_key(|candidate| canonical_key(creature, candidate));
    perceived.truncate(ResourceLimit::PerceptionCandidates.maximum());
    perceived
}

/// §2.1 item 3. `sense_invisible` is read and unused until invisibility exists.
#[must_use]
pub fn is_eligible(
    targeting: &ProjectV2Targeting,
    creature: MovementLocalPosition,
    candidate: &TargetCandidate,
) -> bool {
    let _ = targeting.sense_invisible;
    targeting.hostile
        && targeting.can_target
        && candidate.eligible
        && candidate.position.floor == creature.floor
}

/// The strategy a `draw` selects by weight, walking nearest, lowest health, most damage, random.
/// No weights, or all zero, is nearest.
#[must_use]
pub fn pick_strategy(weights: Option<ProjectV2StrategyWeights>, draw: u64) -> Strategy {
    let Some(weights) = weights else {
        return Strategy::Nearest;
    };
    let ordered = [
        (Strategy::Nearest, weights.nearest),
        (Strategy::LowestHealth, weights.health),
        (Strategy::MostDamage, weights.damage),
        (Strategy::Random, weights.random),
    ];
    let total = ordered
        .iter()
        .map(|(_, weight)| u64::from(*weight))
        .sum::<u64>();
    if total == 0 {
        return Strategy::Nearest;
    }
    let mut point = draw % total;
    for (strategy, weight) in ordered {
        let weight = u64::from(weight);
        if point < weight {
            return strategy;
        }
        point -= weight;
    }
    Strategy::Nearest
}

/// Selects one of `eligible` (canonically ordered) by `strategy`. Ties break by Chebyshev
/// distance, then the canonical order; `random_draw` indexes the canonical order.
#[must_use]
pub fn select(
    strategy: Strategy,
    creature: MovementLocalPosition,
    eligible: &[TargetCandidate],
    random_draw: u64,
) -> Option<ExactActorRef> {
    let ranked = eligible.iter().enumerate().map(|(order, candidate)| {
        (
            order,
            plane_distance(creature, candidate.position),
            candidate,
        )
    });
    let chosen = match strategy {
        Strategy::Nearest => ranked.min_by_key(|(order, distance, _)| (*distance, *order)),
        Strategy::LowestHealth => {
            ranked.min_by_key(|(order, distance, candidate)| (candidate.health, *distance, *order))
        }
        Strategy::MostDamage => ranked.min_by_key(|(order, distance, candidate)| {
            (std::cmp::Reverse(candidate.damage), *distance, *order)
        }),
        Strategy::Random => {
            let count = u64::try_from(eligible.len()).ok()?;
            if count == 0 {
                return None;
            }
            let index = usize::try_from(random_draw % count).ok()?;
            return eligible.get(index).map(|candidate| candidate.actor);
        }
    };
    chosen.map(|(_, _, candidate)| candidate.actor)
}
