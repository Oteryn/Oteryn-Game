//! ADMIT-CAP-2: the capability predicate for positively identified player fields and walls.
//!
//! D1132 consumes ADMIT-CAP-1's immutable boot binding, never a session's optional selection
//! or a new capability closure. D1140 leaves the real caller with #2187 at the first A2 slot.
//! The caller must independently resolve current scope/generation/policy under the existing
//! authority locks and call again at the new-write boundary. A prepared match is not authority.
//!
//! This entry is only for player-origin field/wall operations. Monster/system sources and
//! unrelated spells keep their own admitted paths; absence of player metadata is no exemption.
//! Refusal is an internal typed result, not a new protocol/error-registry allocation. The
//! transaction owner must map it into definite rollback without a failure receipt or cost.

use super::world_required_capabilities::BoundRequiredCapabilities;
use crate::foundation::RuntimeScopeRefV1;
use oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerFieldCapabilityRefusal {
    StaleScopeBinding,
    WorldInteractionRequired,
}

/// Capability sufficiency only, not permission to mutate, install, recover or publish a field.
/// Missing owner binding must be refused by the caller before this borrowed entry can be used.
/// Historical committed receipts remain classified by the existing transaction owner; they
/// must not acquire new-write authority through this predicate.
///
/// D1132/D1083: WORLD_SPATIAL_FIELDS has no allocated ID and contributes no requirement here.
/// WORLD_INTERACTION's dependencies are already validated by the boot owner; do not recompute
/// closure or infer FIELD from spatial entities, map view or another numeric capability.
pub(crate) fn check_player_field_capabilities(
    required: &BoundRequiredCapabilities,
    current_scope: RuntimeScopeRefV1,
    current_generation: u64,
    current_world_policy_revision: &str,
) -> Result<(), PlayerFieldCapabilityRefusal> {
    if !required.matches_current(
        current_scope,
        current_generation,
        current_world_policy_revision,
    ) {
        return Err(PlayerFieldCapabilityRefusal::StaleScopeBinding);
    }
    if required
        .effective()
        .binary_search(&CAPABILITY_WORLD_INTERACTION_V1)
        .is_err()
    {
        return Err(PlayerFieldCapabilityRefusal::WorldInteractionRequired);
    }
    Ok(())
}

#[cfg(test)]
#[path = "world_required_field_guard_tests.rs"]
mod tests;
