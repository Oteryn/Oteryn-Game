//! Exact live-captured native teleport occurrence; ordinary relocation stays live-only.
use super::*;
use oteryn_gameplay_content::content::native_monster_callbacks::QualifiedNativeCallback;
use oteryn_gameplay_foundation::foundation::RuntimeWorkStamp;

#[derive(Debug, Clone)]
pub struct NativeRetainedCallbackTeleport {
    actor: ExactActorRef,
    stamp: RuntimeWorkStamp,
    activation: u64,
    pins: [[u8; 32]; 4],
    source: QualifiedNativeCallback,
}
impl NativeRetainedCallbackTeleport {
    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub fn matches_current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime
            .owner_fence()
            .is_ok_and(|owner| owner.accepts_stamp(self.stamp))
            && self.activation == pin.activation_sequence()
            && self.pins
                == [
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    pin.map_revision_digest(),
                ]
            && self.source.content() == self.pins[0]
            && runtime
                .source_creature_is_registered(self.actor)
                .unwrap_or(false)
    }
}
impl ChannelRuntimeV1 {
    /// Capture the issuer of an actual source teleport while alive. This proof
    /// cannot authorize a different actor or a new callback after death.
    pub fn capture_native_retained_callback_teleport(
        &self,
        actor: ExactActorRef,
        source: &QualifiedNativeCallback,
        stamp: RuntimeWorkStamp,
    ) -> Result<NativeRetainedCallbackTeleport, CarrierError> {
        let entry = source.entry();
        if !matches!(
            entry.ability.key.as_str(),
            "oteryn:ability.spell.maxxenteleport" | "oteryn:ability.spell.zamulosh_tp"
        ) || source.content() != self.content_pin().server_artifact_digest()
            || !self.owner_fence()?.accepts_stamp(stamp)
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let current = self.companion_snapshot(actor)?;
        if current.state.policy.definition_key != entry.creature.key
            || current.state.policy.definition_revision != entry.creature.revision
            || current.state.master.is_some()
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let pin = self.content_pin();
        Ok(NativeRetainedCallbackTeleport {
            actor,
            stamp,
            activation: pin.activation_sequence(),
            pins: [
                pin.server_artifact_digest(),
                pin.client_artifact_digest(),
                pin.frame_binding_digest(),
                pin.map_revision_digest(),
            ],
            source: source.clone(),
        })
    }
    /// Source-only compare commit over the same canonical position/stack owner.
    /// Ordinary relocation remains live-only; the retained proof must stay current.
    pub fn commit_native_retained_callback_teleport(
        &mut self,
        source: &NativeRetainedCallbackTeleport,
        expected: MovementPositionSnapshot,
        next: MovementLocalPosition,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        if !source.matches_current(self) {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        if source.actor.0 != expected.0.actor_ref {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        if self.contains_live_creature(source.actor) {
            return self.borrow_movement_position().commit_creature_relocation(
                source.actor,
                expected,
                next,
            );
        }
        self.carrier
            .compare_commit_registered_source_position(
                &self.continuity,
                expected.0,
                expected.0.version.context,
                LocalPosition {
                    x: next.x,
                    y: next.y,
                    floor: next.floor,
                },
            )
            .map(MovementPositionSnapshot)
    }
}

#[cfg(test)]
#[path = "native_retained_teleport_tests.rs"]
mod tests;
