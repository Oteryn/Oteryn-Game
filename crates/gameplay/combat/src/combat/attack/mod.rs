//! ATTACK-1a: the pure half of ATTACK-1 (ATTACK-0 §4 and §5,
//! `docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md`).
//!
//! The attack-target owner state, the auto-attack timer, the in-fight deadline, the fist, melee
//! and creature-melee formulas, the block budget and the constants table. ATTACK-1b wires the
//! target, the timer, the in-fight deadline and the fist formula into the Channel runtime
//! (`gameplay_transport::attack`) and the GAME-ABILITY-01 pipeline; ATTACK-2 composes weapon
//! melee and the creature's block budget, defence and armor there.

mod chase;

pub use chase::{
    CHASE_SEARCHES_PER_WINDOW, CHASE_WINDOW_MICROS, CREATURE_COST, ChaseDecision,
    ChaseSearchBudget, ChaseState, chase_search_order,
};
pub use oteryn_gameplay_combat_rules::attack::block::BlockBudget;
pub use oteryn_gameplay_combat_rules::attack::constants::{AttackConstants, FightMode};
pub use oteryn_gameplay_combat_rules::attack::formulas::{
    armor_reduction_bounds, creature_melee_formula, player_fist_formula, player_melee_formula,
};
pub use oteryn_gameplay_combat_rules::attack::target::{
    AttackState, SwingPoll, TargetFacts, TargetRefusal,
};
