//! ATTACK-1a: the pure half of ATTACK-1 (ATTACK-0 §4 and §5,
//! `docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md`).
//!
//! The attack-target owner state, the auto-attack timer, the in-fight deadline, the fist, melee
//! and creature-melee formulas, the block budget and the constants table. ATTACK-1b wires the
//! target, the timer, the in-fight deadline and the fist formula into the Channel runtime
//! (`gameplay_transport::attack`) and the GAME-ABILITY-01 pipeline; weapon melee, creature
//! defence, armor and the block budget are not composed yet.

#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "ATTACK-1b composes fists only; weapon melee, creature defence, armor and block are follow-ups"
    )
)]
mod block;
mod constants;
mod formulas;
mod target;
#[cfg(test)]
mod tests;

#[cfg(test)]
use block::BlockBudget;
pub(crate) use constants::{AttackConstants, FightMode};
pub(crate) use formulas::player_fist_formula;
#[cfg(test)]
use formulas::{
    DefenceSource, armor_reduction_bounds, creature_melee_formula, defence_formula,
    defence_mode_factor, player_melee_formula, zero_skill_defence_bounds,
};
pub(crate) use target::{AttackState, SwingPoll, TargetFacts, TargetRefusal};
#[cfg(test)]
use target::{SwingWait, TargetCleared, TargetValidity};
