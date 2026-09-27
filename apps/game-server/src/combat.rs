//! Nonshipping fixed-one-creature Combat structural boundary.
//!
//! Canary and CrystalServer informed only the high-level lethal -> death ->
//! corpse ordering. Oteryn identity, authority, replay and resource limits are
//! defined here by the current Channel owner and its exact Ability commit.

use crate::foundation::{
    CarrierError, CurrentOwnerCombatDeath, ExactActorRef, RuntimeCorpseProjection,
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
