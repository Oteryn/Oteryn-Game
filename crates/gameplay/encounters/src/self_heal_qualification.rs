//! Typed loaded-document membership qualification. No digest->membership inference.
use crate::content::ProjectV2Draft;
pub use oteryn_gameplay_encounter_rules::self_heal::QualifiedCreatureSelfHeal as QualifiedSelfHealRules;

/// Runtime adapter for pure loaded-document qualification.
#[derive(Debug, Clone)]
pub struct QualifiedCreatureSelfHeal(QualifiedSelfHealRules);

impl QualifiedCreatureSelfHeal {
    pub fn qualify(
        draft: &ProjectV2Draft,
        target: &str,
        ability: &str,
    ) -> Result<Self, &'static str> {
        QualifiedSelfHealRules::qualify(draft, target, ability).map(Self)
    }

    pub fn removes_paralysis(&self) -> bool {
        self.0.removes_paralysis()
    }

    pub fn magnitude_range(&self) -> (u64, u64) {
        self.0.magnitude_range()
    }

    pub fn source_schedule(&self) -> (u64, u32) {
        self.0.source_schedule()
    }

    pub fn register(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
    ) -> Result<crate::foundation::CreatureSelfHealRegistration, crate::foundation::CarrierError>
    {
        let (target, ability, max_health, minimum, maximum) = self.0.registration_inputs();
        runtime.bind_creature_self_heal(target, ability, max_health, minimum, maximum)
    }
}
