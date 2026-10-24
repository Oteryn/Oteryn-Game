//! Runtime target resolution; authored restrictions remain Content-owned.
pub use oteryn_gameplay_content::content::spell_data::target::*;

pub trait AllowedTargetsExt {
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
pub struct CastTarget {
    /// Creature id of the caster.
    pub caster: u64,
    /// Creature id of the target.
    pub creature: u64,
    /// Exact actor atom of the target in the Ability pipeline.
    pub actor: String,
    /// The creature that summoned or convinced the target; `None` for a creature without a master.
    pub master: Option<u64>,
}
