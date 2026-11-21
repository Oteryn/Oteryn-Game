//! Crystal 201189f hazard_primal.lua:60-70: forced masterless Fungosaurus.
use super::*;

impl ChannelRuntimeV1 {
    /// Called only by the source-qualified, committed Pod callback consumer.
    /// The callback's creator may have died; its scoped position remains the
    /// source. Known current Map evidence is mandatory even with force=true.
    pub fn spawn_primal_pod_fungosaurus(
        &mut self,
        at: MovementLocalPosition,
        map_exists: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        self.owner_fence().map_err(EncounterSpawnError::Native)?;
        if !matches!(
            self.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || at.floor != 15
            || !(33547..=33570).contains(&at.x)
            || !(32749..=32769).contains(&at.y)
        {
            return Err(EncounterSpawnError::Native(CarrierError::WrongScope));
        }
        let key = "oteryn:creature.fungosaurus";
        let revision = self
            .companion_policy(key)
            .map_err(EncounterSpawnError::Native)?
            .definition_revision
            .clone();
        self.admit_native_masterless_with_placement(
            key,
            &revision,
            self.content_pin().server_artifact_digest(),
            at,
            true,
            map_exists,
        )
    }
}
