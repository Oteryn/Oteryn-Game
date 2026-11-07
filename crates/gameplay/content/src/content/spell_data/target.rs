//! Immutable authored target restrictions.
/// `targeting.allowed_targets`; `Any` when the field is absent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AllowedTargets {
    #[default]
    Any,
    SelfOnly,
    SelfOrOwnSummons,
    /// Any creature but the caster (B.5, the `caster_restriction` target rule `not_self`).
    NotSelf,
}

impl AllowedTargets {
    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "any" => Self::Any,
            "self_only" => Self::SelfOnly,
            "self_or_own_summons" => Self::SelfOrOwnSummons,
            "not_self" => Self::NotSelf,
            _ => return None,
        })
    }
}
