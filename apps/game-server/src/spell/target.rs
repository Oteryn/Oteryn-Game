//! Who a targeted cast may be aimed at, `targeting.allowed_targets`
//! (`docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` part D.3, owner S27).
//!
//! The healing runes may be used only on the caster or on the caster's own summoned or convinced
//! creatures (D.3.1 step 1, the F reading of Q7). The Target Resolver names the creature and its
//! master in a [`CastTarget`]; no summon owner exists yet, so every live creature has no master
//! and only the caster qualifies.

/// `targeting.allowed_targets`; `Any` when the field is absent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AllowedTargets {
    #[default]
    Any,
    SelfOnly,
    SelfOrOwnSummons,
}

impl AllowedTargets {
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "any" => Self::Any,
            "self_only" => Self::SelfOnly,
            "self_or_own_summons" => Self::SelfOrOwnSummons,
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
        }
    }
}

/// The creature a targeted cast is aimed at, as the Target Resolver resolved it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CastTarget {
    /// Creature id of the caster.
    pub(crate) caster: u64,
    /// Creature id of the target.
    pub(crate) creature: u64,
    /// The creature that summoned or convinced the target; `None` for a creature without a master.
    pub(crate) master: Option<u64>,
}
