//! Keep a physical boss inert until its exact durable realization is known.
use super::*;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_entities::carrier::runtime_actor_source_reservation::{
    ScopeCreatureReservationProof, source_reservation_seal,
};

pub(super) struct RealizationGuard {
    actor: ExactActorRef,
    arrival: [u8; 16],
    creature: &'static str,
    content: [u8; 32],
}
impl source_reservation_seal::Sealed for RealizationGuard {}
impl ScopeCreatureReservationProof for RealizationGuard {
    fn scope(&self) -> (RuntimeScopeRefV1, ScopeOwnershipGeneration) {
        (self.actor.runtime_scope(), self.actor.scope_generation())
    }
    fn event_identity(&self) -> [u8; 16] {
        self.arrival
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"oteryn:m49:realization/v1");
        hash.update(self.arrival);
        hash.update(self.creature.as_bytes());
        hash.update(self.content);
        hash.update(self.actor.0.actor_local_id.0.to_be_bytes());
        hash.update(self.actor.0.actor_local_generation.0.to_be_bytes());
        hash.finalize().into()
    }
    fn affected_creatures(&self) -> &[ExactActorRef] {
        std::slice::from_ref(&self.actor)
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime.encounter_owner_scope() == self.scope()
            && runtime.content_pin().server_artifact_digest() == self.content
            && runtime.matches_live_creature_identity(self.actor, self.creature.as_bytes())
    }
}
impl RealizationGuard {
    pub(super) fn new(
        actor: ExactActorRef,
        arrival: [u8; 16],
        creature: &'static str,
        content: [u8; 32],
    ) -> Self {
        Self {
            actor,
            arrival,
            creature,
            content,
        }
    }
    pub(super) fn hold(&self, runtime: &mut ChannelRuntimeV1) -> bool {
        let Ok(mut reservation) = runtime.prepare_scope_creature_reservation(self) else {
            return false;
        };
        runtime
            .reserve_scope_creatures(&mut reservation, self)
            .is_ok()
    }
    /// Called only after successful ACK or an independently fresh committed ACK read.
    /// Reconstruct the exact source reservation even if the volatile actor memo was lost.
    pub(super) fn release(&self, runtime: &mut ChannelRuntimeV1) {
        if !runtime.actor_spell_reserved(self.actor) {
            return;
        }
        let Ok(reservation) = runtime.prepare_scope_creature_reservation(self) else {
            return;
        };
        let _ = runtime.release_scope_creatures_after_observed_outcome(reservation, self);
    }
}
