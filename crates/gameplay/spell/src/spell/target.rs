//! Runtime checked-target witness for the cast admission adapter.
pub use oteryn_gameplay_spell_rules::target::*;

pub(super) trait CastTargetExt {
    fn checked(&self) -> CheckedTarget;
}

impl CastTargetExt for CastTarget {
    /// The target once the cast has been checked against it.
    fn checked(&self) -> CheckedTarget {
        CheckedTarget {
            creature: self.creature,
            actor: self.actor.clone(),
        }
    }
}

/// The target an accepted cast was checked against. Only the cast check makes one, so the plan
/// of a [`super::CastResolution`] applies to the creature the check allowed and to no other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedTarget {
    creature: u64,
    actor: String,
}

impl CheckedTarget {
    pub fn creature(&self) -> u64 {
        self.creature
    }

    pub fn actor(&self) -> &str {
        &self.actor
    }
}
