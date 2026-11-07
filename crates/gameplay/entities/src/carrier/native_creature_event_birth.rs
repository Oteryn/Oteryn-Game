//! A real CreatureEvent may retire its parent before source Game.createMonster.
//! Capture the closed event source while that exact actor still exists, then
//! reuse the one physical admission owner under independently current fences.
use super::*;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_content::content::native_monster_callbacks::{
    NativeOutburstEvent, QualifiedNativeCreatureEventSpawn,
};

#[derive(Debug)]
pub struct PreparedNativeCreatureEventBirth {
    source: QualifiedNativeCreatureEventSpawn,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    activation: u64,
    pins: [[u8; 32]; 4],
    stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
    admitted: Option<ExactActorRef>,
}
impl ChannelRuntimeV1 {
    /// This creates no actor and grants no map position. The ordinary onThink
    /// source must still be live; the death source needs the canonical lethal
    /// receipt of the actual Charging Outburst incarnation, never just HP0.
    pub fn prepare_native_creature_event_birth(
        &self,
        native: &NativeGameplayState,
        stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
        parent: ExactActorRef,
        source: &QualifiedNativeCreatureEventSpawn,
    ) -> Result<PreparedNativeCreatureEventBirth, CarrierError> {
        if !self.owner_fence()?.accepts_stamp(stamp)
            || native.source_digest() != self.content_pin().server_artifact_digest()
            || source.source_digest() != native.source_digest()
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let snapshot = self.companion_snapshot_including_dead(parent)?;
        if snapshot.state.policy.definition_key != source.parent().key
            || snapshot.state.policy.definition_revision != source.parent().revision
            || snapshot.state.master.is_some()
            || self.native_summon_master(parent).is_some()
            || !native
                .policies()
                .policies()
                .iter()
                .any(|policy| policy.as_ref() == snapshot.state.policy.as_ref())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        match source.event() {
            NativeOutburstEvent::Think
            | NativeOutburstEvent::OmrafirBoom
            | NativeOutburstEvent::OmrafirReform
                if snapshot.health > 0 => {}
            NativeOutburstEvent::ChargingDeath if snapshot.health == 0 => {
                self.carrier
                    .committed_lethal_receipt_inner(&self.continuity, parent.0)?;
            }
            _ => return Err(CarrierError::CreatureNotActionable),
        }
        let child = self.companion_policy(&source.child().key)?;
        if child.definition_revision != source.child().revision
            || !native
                .policies()
                .policies()
                .iter()
                .any(|policy| policy.as_ref() == child.as_ref())
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        Ok(PreparedNativeCreatureEventBirth {
            source: source.clone(),
            scope: self.binding().runtime_scope(),
            generation: self.binding().scope_generation(),
            activation: self.content_pin().activation_sequence(),
            pins: [
                native.source_digest(),
                self.content_pin().client_artifact_digest(),
                self.content_pin().frame_binding_digest(),
                self.content_pin().map_revision_digest(),
            ],
            stamp,
            admitted: None,
        })
    }
    /// The opaque source captured above survives administrative parent removal.
    /// A first success is retained as data, so a retry never creates a second child.
    #[allow(
        clippy::too_many_arguments,
        reason = "current native owner, prepared event and independently qualified placement are separate proofs"
    )]
    pub fn admit_prepared_native_creature_event_birth(
        &mut self,
        native: &NativeGameplayState,
        prepared: &mut PreparedNativeCreatureEventBirth,
        requested_center: MovementLocalPosition,
        at: MovementLocalPosition,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        use EncounterSpawnError as E;
        if !self
            .owner_fence()
            .map_err(E::Native)?
            .accepts_stamp(prepared.stamp)
            || self.binding().runtime_scope() != prepared.scope
            || self.binding().scope_generation() != prepared.generation
            || self.content_pin().activation_sequence() != prepared.activation
            || [
                native.source_digest(),
                self.content_pin().client_artifact_digest(),
                self.content_pin().frame_binding_digest(),
                self.content_pin().map_revision_digest(),
            ] != prepared.pins
            || self.content_pin().server_artifact_digest() != prepared.source.source_digest()
        {
            return Err(E::StaleOwner);
        }
        if let Some(actor) = prepared.admitted {
            return Ok(actor);
        }
        let source = &prepared.source;
        let child = self
            .companion_policy(&source.child().key)
            .map_err(E::Native)?;
        if child.definition_revision != source.child().revision
            || !native
                .policies()
                .policies()
                .iter()
                .any(|policy| policy.as_ref() == child.as_ref())
        {
            return Err(E::Native(CarrierError::CreatureTargetMismatch));
        }
        let occupied_allowed = if source.forced() && at == requested_center {
            true
        } else {
            if !self
                .current_native_source_occupancy(native, &source.child().key, at, None)
                .map_err(E::Native)?
                .admits_occupants
            {
                return Err(E::RefusedMap);
            }
            true
        };
        let actor = self.admit_native_masterless_with_placement(
            &source.child().key,
            &source.child().revision,
            source.source_digest(),
            at,
            occupied_allowed,
            map_admits,
        )?;
        prepared.admitted = Some(actor);
        Ok(actor)
    }
}
