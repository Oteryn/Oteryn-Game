//! Who a targeted cast may be aimed at, `targeting.allowed_targets`
//! (`docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` parts D.3 and B.5, owner S27).
//!
//! The healing runes may be used only on the caster or on the caster's own summoned or convinced
//! creatures (D.3.1 step 1, the F reading of Q7). Nature's Embrace may be aimed at anyone but the
//! caster (B.5 step 1, `not_self`). The Target Resolver names the creature and its master in a
//! [`CastTarget`]; no summon owner exists yet, so every live creature has no master and only the
//! caster qualifies for the runes.

/// `targeting.allowed_targets`; `Any` when the field is absent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AllowedTargets {
    #[default]
    Any,
    SelfOnly,
    SelfOrOwnSummons,
    /// Any creature but the caster (B.5, the `caster_restriction` target rule `not_self`).
    NotSelf,
}

impl AllowedTargets {
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "any" => Self::Any,
            "self_only" => Self::SelfOnly,
            "self_or_own_summons" => Self::SelfOrOwnSummons,
            "not_self" => Self::NotSelf,
            _ => return None,
        })
    }

    /// Whether a cast may be aimed at `target`.
    pub(crate) fn allows(self, target: &CastTarget) -> bool {
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
