//! D1208 closed durability-facing predicate. No gameplay/transport type enters durable roots.
use crate::foundation::RuntimeScopeRefV1;

/// A refusal only, never authority or a fallback grant. No Default implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldCapabilityRefused {
    _private: (),
}
impl FieldCapabilityRefused {
    pub(crate) fn refused() -> Self {
        Self { _private: () }
    }
}

/// The World policy key is independently resolved under the actual current-authority locks.
/// The production implementation belongs to the boot-bound owner; test roots may supply
/// explicit test-only pass/refuse implementations. No production no-op implementation exists.
pub(crate) trait PlayerFieldCapabilityCheck: Send + Sync {
    fn check_player_field(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
        world_policy_revision: &str,
    ) -> Result<(), FieldCapabilityRefused>;
}
