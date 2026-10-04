//! ATTACK-1a: the pure half of ATTACK-1 (ATTACK-0 §4 and §5,
//! `docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md`).
//!
//! The attack-target owner state, the auto-attack timer, the in-fight deadline, the fist, melee
//! and creature-melee formulas, the block budget and the constants table. Nothing calls it yet;
//! ATTACK-1b wires it into the Channel runtime and the GAME-ABILITY-01 pipeline.

mod block;
mod constants;
mod formulas;
mod target;
#[cfg(test)]
mod tests;

pub(crate) use block::BlockBudget;
pub(crate) use constants::{AttackConstants, FightMode};
pub(crate) use formulas::{
    DefenceSource, armor_reduction_bounds, creature_melee_formula, defence_formula,
    defence_mode_factor, player_fist_formula, player_melee_formula, zero_skill_defence_bounds,
};
pub(crate) use target::{
    AttackState, MELEE_RANGE, Swing, SwingPoll, SwingRngPurpose, SwingWait, TargetCleared,
    TargetFacts, TargetRefusal, TargetValidity,
};
