//! ADMIT0 §6: immutable requirements are useful only for the independently current owner.
use super::world_required_capabilities::BoundRequiredCapabilities;
use crate::durability::spell_items_abi::SpellFieldOrigin;
use crate::foundation::RuntimeScopeRefV1;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FieldCapabilityRefusal {
    StaleBinding,
    MissingCapability(u32),
}

pub(super) fn check(
    bound: &BoundRequiredCapabilities,
    scope: RuntimeScopeRefV1,
    generation: u64,
    policy: &str,
) -> Result<(), FieldCapabilityRefusal> {
    if !matches!(scope, RuntimeScopeRefV1::Channel { .. })
        || !bound.matches_current(scope, generation, policy)
    {
        return Err(FieldCapabilityRefusal::StaleBinding);
    }
    for capability in [11, 30] {
        if bound.effective().binary_search(&capability).is_err() {
            return Err(FieldCapabilityRefusal::MissingCapability(capability));
        }
    }
    Ok(())
}

pub(super) fn player_origin(bound: &Arc<BoundRequiredCapabilities>) -> SpellFieldOrigin {
    SpellFieldOrigin::Player(current_check(bound))
}

pub(super) fn current_check(
    bound: &Arc<BoundRequiredCapabilities>,
) -> crate::durability::fresh_admission::CurrentCapabilityCheck {
    let bound = bound.clone();
    Arc::new(move |scope, generation, policy| check(&bound, scope, generation, policy).is_ok())
}

#[cfg(test)]
#[path = "world_required_field_guard_tests.rs"]
mod tests;
