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

    /// Qualify a native-only body's exact closure through the current opaque loader state.
    pub fn qualify_current_native(
        runtime: &oteryn_gameplay_entities::ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        target: &str,
        ability: &str,
    ) -> Result<Self, &'static str> {
        runtime.owner_fence().map_err(|_| "current native owner")?;
        if runtime.content_pin().server_artifact_digest() != native.source_digest() {
            return Err("detached native content");
        }
        let row = native
            .creature_profiles()
            .records
            .iter()
            .find(|row| row.profile.target.key == target)
            .ok_or("loaded native Creature")?;
        let closure = row
            .attack_definitions
            .as_ref()
            .ok_or("loaded native closure")?;
        QualifiedSelfHealRules::qualify_native_parts(
            &closure.records,
            &closure.profiles,
            target,
            ability,
        )
        .map(Self)
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
        runtime: &oteryn_gameplay_entities::ChannelRuntimeV1,
    ) -> Result<
        oteryn_gameplay_entities::CreatureSelfHealRegistration,
        oteryn_gameplay_domain::domain::carrier_values::CarrierError,
    > {
        let (target, ability, max_health, minimum, maximum) = self.0.registration_inputs();
        runtime.bind_creature_self_heal(target, ability, max_health, minimum, maximum)
    }
}
