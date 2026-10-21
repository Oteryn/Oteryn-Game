//! Runtime target resolution; authored restrictions remain Content-owned.
pub(crate) use crate::content::spell_data::target::*;

pub(crate) trait AllowedTargetsExt {
    fn allows(self, target: &CastTarget) -> bool;
}

impl AllowedTargetsExt for AllowedTargets {
    /// Whether a cast may be aimed at `target`.
    fn allows(self, target: &CastTarget) -> bool {
        let own = target.creature == target.caster;
        match self {
            Self::Any => true,
            Self::SelfOnly => own,
            Self::SelfOrOwnSummons => own || target.master == Some(target.caster),
            Self::NotSelf => !own,
        }
    }
}

/// The creature a targeted cast is aimed at, as the Target Resolver resolved it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CastTarget {
    /// Creature id of the caster.
    pub(crate) caster: u64,
    /// Creature id of the target.
    pub(crate) creature: u64,
    /// Exact actor atom of the target in the Ability pipeline.
    pub(crate) actor: String,
    /// The creature that summoned or convinced the target; `None` for a creature without a master.
    pub(crate) master: Option<u64>,
}

impl CastTarget {
    /// The target once the cast has been checked against it.
    pub(super) fn checked(&self) -> CheckedTarget {
        CheckedTarget {
            creature: self.creature,
            actor: self.actor.clone(),
        }
    }
}

/// The target an accepted cast was checked against. Only the cast check makes one, so the plan
/// of a [`super::CastResolution`] applies to the creature the check allowed and to no other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckedTarget {
    creature: u64,
    actor: String,
}

impl CheckedTarget {
    pub(crate) fn creature(&self) -> u64 {
        self.creature
    }

    pub(crate) fn actor(&self) -> &str {
        &self.actor
    }
}
