//! The installed physical owner, without copying or exporting its non-clonable fence.
use super::*;

impl ChannelRuntimeV1 {
    pub fn admit_current_native_encounter_masterless(
        &mut self,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        parent: ExactActorRef,
        spec: &NativeEncounterSpawnSpec,
        at: MovementLocalPosition,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        if !self
            .owner_fence()
            .map_err(EncounterSpawnError::Native)?
            .accepts_stamp(stamp)
        {
            return Err(EncounterSpawnError::StaleOwner);
        }
        if spec.content != self.content_pin().server_artifact_digest()
            || !self.matches_live_creature_identity(parent, spec.parent.as_bytes())
        {
            return Err(EncounterSpawnError::Native(
                CarrierError::CreatureTargetMismatch,
            ));
        }
        self.admit_native_masterless_at(
            &spec.child,
            &spec.child_revision,
            spec.content,
            at,
            map_admits,
        )
    }

    pub fn remove_current_native_encounter_subject(
        &mut self,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        actor: ExactActorRef,
        expected_key: &str,
        expected_content: [u8; 32],
    ) -> Result<(), CarrierError> {
        if !self.owner_fence()?.accepts_stamp(stamp) {
            return Err(CarrierError::WrongScope);
        }
        self.remove_native_masterless_subject(actor, expected_key, expected_content)
    }
}
