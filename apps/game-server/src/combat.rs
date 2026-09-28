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
// `mod loot_plan;` resolves against that site's own directory in that
// context; the explicit path keeps both inclusions pointing at the same file.
#[path = "combat/loot_plan.rs"]
mod loot_plan;

// D2a is planning-only (no durability, no foundation wiring, no XP); D2b
// wires a real committed death and `durability::item_mint` into this API.
#[allow(
    unused_imports,
    reason = "D2a has no production caller yet; D2b wires one"
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
