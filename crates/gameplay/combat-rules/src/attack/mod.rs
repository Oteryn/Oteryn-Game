//! ATTACK-1a: the pure half of ATTACK-1 (ATTACK-0 §4 and §5,
//! `docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md`).
//!
//! The attack-target owner state, the auto-attack timer, the in-fight deadline, the fist, melee
//! and creature-melee formulas, the block budget and the constants table. ATTACK-1b wires the
//! target, the timer, the in-fight deadline and the fist formula into the Channel runtime
//! (`gameplay_transport::attack`) and the GAME-ABILITY-01 pipeline; ATTACK-2 composes weapon
//! melee and the creature's block budget, defence and armor there.

pub mod block;
pub mod chase;
pub mod constants;
pub mod formulas;
#[cfg(test)]
mod parity_tests;
pub mod ranged;
pub mod target;
#[cfg(test)]
mod tests;

pub use block::BlockBudget;
pub use constants::{AttackConstants, FightMode};
#[cfg(test)]
use formulas::{DefenceSource, defence_formula, defence_mode_factor, zero_skill_defence_bounds};
pub use formulas::{
    armor_reduction_bounds, creature_melee_formula, player_fist_formula, player_melee_formula,
};
pub use target::{AttackState, SwingPoll, TargetFacts, TargetRefusal};
#[cfg(test)]
use target::{SwingWait, TargetCleared, TargetValidity};

#[cfg(test)]
mod formulas_property_tests;
