//! Nonshipping fixed-one-creature Combat structural boundary.
//!
//! Canary and CrystalServer informed only the high-level lethal -> death ->
//! corpse ordering. Oteryn identity, authority, replay and resource limits are
//! defined here by the current Channel owner and its exact Ability commit.

use crate::foundation::{
    CarrierError, CurrentOwnerCombatDeath, ExactActorRef, RuntimeCorpseProjection,
};

// Explicit `#[path]`: `combat.rs` is also compiled under a different module
// name via `foundation/mod.rs`'s `#[path = "../combat.rs"] mod
// exact_actor_test_combat;` (standalone Foundation test crate). An unpathed
// `mod loot_plan;`/`mod death_reward;` resolves against that site's own
// directory in that context; the explicit path keeps both inclusions
// pointing at the same file.
#[path = "combat/death_reward.rs"]
mod death_reward;
#[path = "combat/loot_plan.rs"]
mod loot_plan;

// D2a/D2b have no production caller yet (protocol/admission composition is a
// later, separate stage), so nothing outside this crate's tests reaches
// these re-exports today.
#[allow(
    unused_imports,
    reason = "no production caller yet; a later admission stage wires one"
)]
pub(crate) use death_reward::{
    COMBAT01_INFLIGHT_LOOT_MINTS_PER_SCOPE_MAX, COMBAT01_REWARD_PRINCIPALS_MAX,
    COMBAT01_XP_DESCENDANTS_PER_DEATH_MAX, CombatDeathRewardLootError, CombatDeathRewardXpError,
    CombatResourceLimitError, CreatureDeathRewardAdmissionError, CreatureDeathRewardInput,
    CreatureDeathRewardOutcome, DeathGroundContext, DurabilitySession, RewardPrincipal,
    RewardProgressionBinding, check_inflight_loot_mint_capacity, check_reward_principal_count,
    settle_creature_death_rewards,
};
#[allow(
    unused_imports,
    reason = "no production caller yet; a later admission stage wires one"
)]
pub(crate) use loot_plan::{
    COMBAT01_LOOT_PLAN_BYTES_MAX, COMBAT01_LOOT_PLAN_ENTRIES_MAX, COMBAT01_LOOT_PLAN_ITEMS_MAX,
    COMBAT01_LOOT_RNG_DRAWS_MAX, LootDefinitionRef, LootPlan, LootPlanDeathKey, LootPlanEntry,
    LootPlanError, LootSelectionAlgorithm, LootTableDefinition, LootTableEntry, plan_creature_loot,
};

/// Project the already committed lethal transition. Combat cannot manufacture
/// lethality, actor identity or position: it can only ask the physical owner
/// for an opaque receipt and immediately return that receipt for projection.
pub(crate) fn project_fixed_one_creature_death<'a>(
    owner: &'a mut CurrentOwnerCombatDeath<'_>,
    actor: ExactActorRef,
) -> Result<&'a RuntimeCorpseProjection, CarrierError> {
    let receipt = owner.committed_lethal_receipt(actor)?;
    owner.project_committed_lethal(receipt)
}
