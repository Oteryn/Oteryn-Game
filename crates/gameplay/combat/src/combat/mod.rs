//! Nonshipping fixed-one-creature Combat structural boundary.
//!
//! Canary and CrystalServer informed only the high-level lethal -> death ->
//! corpse ordering. Oteryn identity, authority, replay and resource limits are
//! defined here by the current Channel owner and its exact Ability commit.

// A directory module keeps one child-module layout in native and standalone test roots.
pub mod chain;
pub mod charm_effects;
mod death_reward;
pub mod formula;
mod loot_plan;

// The live kill-reward caller (`gameplay_transport::kill_reward`) uses only
// part of these re-exports; the rest serve the path-loaded test harnesses.
#[allow(
    unused_imports,
    reason = "partly used by the live caller; the rest serve path-loaded test crates"
)]
pub use death_reward::{
    COMBAT01_INFLIGHT_LOOT_MINTS_PER_SCOPE_MAX, COMBAT01_REWARD_PRINCIPALS_MAX,
    COMBAT01_XP_DESCENDANTS_PER_DEATH_MAX, CapturedRewardPrincipal, CombatBestiaryOutcome,
    CombatDeathRewardBestiaryError, CombatDeathRewardLootError, CombatDeathRewardXpError,
    CombatResourceLimitError, CommittedCorpseLoot, CreatureDeathBestiaryInput,
    CreatureDeathRewardAdmissionError, CreatureDeathRewardInput, CreatureDeathRewardOutcome,
    CreatureDeathRewardWithBestiaryOutcome, DeathGroundContext, DurabilitySession,
    GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX, ProjectedCreatureDeathFacts, RewardPrincipal,
    RewardProgressionBinding, capture_projected_death_facts, check_corpse_container_capacity,
    check_inflight_loot_mint_capacity, check_reward_principal_count, loot_plan_seed,
    settle_creature_death_rewards, settle_creature_death_rewards_with_bestiary,
};
#[allow(
    unused_imports,
    reason = "partly used by the live caller; the rest serve path-loaded test crates"
)]
pub use loot_plan::{
    COMBAT01_LOOT_PLAN_BYTES_MAX, COMBAT01_LOOT_PLAN_ENTRIES_MAX, COMBAT01_LOOT_PLAN_ITEMS_MAX,
    COMBAT01_LOOT_RNG_DRAWS_MAX, LootDefinitionRef, LootPlan, LootPlanDeathKey, LootPlanEntry,
    LootPlanError, LootSelectionAlgorithm, LootTableDefinition, LootTableEntry, plan_creature_loot,
};

pub use crate::foundation::project_fixed_one_creature_death;
