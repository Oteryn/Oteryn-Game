//! A live scheduled native Lua Combat retains its exact registered Creature.
//! This does not start a cast, revive an actor, or grant arbitrary spell values.
use super::*;
use oteryn_gameplay_content::content::native_monster_callbacks::{
    QualifiedNativeCallback, QualifiedNativeCallbackStage,
};
use oteryn_gameplay_foundation::foundation::RuntimeWorkStamp;

#[derive(Debug, Clone)]
pub struct NativeRetainedCallbackCaster {
    actor: ExactActorRef,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    stamp: RuntimeWorkStamp,
    activation: u64,
    pins: [[u8; 32]; 4],
    creature: oteryn_gameplay_content::content::ProjectV2DefinitionRef,
    ability: oteryn_gameplay_content::content::ProjectV2DefinitionRef,
    stages: Vec<[u8; 32]>,
}
impl NativeRetainedCallbackCaster {
    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub fn original_work_stamp(&self) -> RuntimeWorkStamp {
        self.stamp
    }
    pub fn matches_stage(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        stage: &QualifiedNativeCallbackStage,
    ) -> bool {
        let pin = runtime.content_pin();
        let entry = stage.parent().entry();
        self.actor == actor
            && self.scope == runtime.binding().runtime_scope()
            && self.generation == runtime.binding().scope_generation()
            && runtime
                .owner_fence()
                .is_ok_and(|fence| fence.accepts_stamp(self.stamp))
            && self.activation == pin.activation_sequence()
            && self.pins
                == [
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    pin.map_revision_digest(),
                ]
            && stage.parent().content() == self.pins[0]
            && entry.creature == self.creature
            && entry.ability == self.ability
            && self.stages.get(stage.index()) == Some(&stage.fingerprint())
            && runtime
                .source_creature_is_registered(actor)
                .unwrap_or(false)
    }
}
impl ChannelRuntimeV1 {
    /// Capture only while the actual native caster is alive, under the same
    /// work stamp that schedules its closed source callback. The old Combat
    /// object remains valid across setType; the exact actor must stay registered.
    pub fn capture_native_retained_callback_caster(
        &self,
        actor: ExactActorRef,
        source: &QualifiedNativeCallback,
        stamp: RuntimeWorkStamp,
    ) -> Result<NativeRetainedCallbackCaster, CarrierError> {
        let fence = self.owner_fence()?;
        if !fence.accepts_stamp(stamp)
            || source.content() != self.content_pin().server_artifact_digest()
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let snapshot = self.companion_snapshot(actor)?;
        let entry = source.entry();
        if snapshot.state.policy.definition_key != entry.creature.key
            || snapshot.state.policy.definition_revision != entry.creature.revision
            || snapshot.state.master.is_some()
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let mut stages = Vec::new();
        stages
            .try_reserve_exact(3)
            .map_err(|_| CarrierError::AllocationFailed)?;
        for index in 0..3 {
            let Some(stage) = source
                .stage(index)
                .map_err(|_| CarrierError::CreatureTargetMismatch)?
            else {
                break;
            };
            stages.push(stage.fingerprint());
        }
        if stages.is_empty() {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let pin = self.content_pin();
        Ok(NativeRetainedCallbackCaster {
            actor,
            scope: self.binding().runtime_scope(),
            generation: self.binding().scope_generation(),
            stamp,
            activation: pin.activation_sequence(),
            pins: [
                pin.server_artifact_digest(),
                pin.client_artifact_digest(),
                pin.frame_binding_digest(),
                pin.map_revision_digest(),
            ],
            creature: entry.creature.clone(),
            ability: entry.ability.clone(),
            stages,
        })
    }
}
