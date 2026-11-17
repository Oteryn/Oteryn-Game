//! Crystal data-global/scripts/quests/a_piece_of_cake/movements_extraction_spot.lua:
//! 26-49 is an actual step-in callback, not a death counter or a position poll.
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementPositionSnapshot, RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
};
use oteryn_gameplay_entities::RaidSpawnIdentity;
use oteryn_gameplay_entities::carrier::runtime_actor_source_reservation::{
    ScopeCreatureReservationProof, source_reservation_seal,
};
use sha2::{Digest, Sha256};

pub const CREATURE: &str = "oteryn:creature.cake_golem";
#[derive(Debug)]
pub struct CakeMachineEntry {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    identity: RaidSpawnIdentity,
    actors: [ExactActorRef; 1],
    position: MovementPositionSnapshot,
    coordinates: [u16; 3],
    event: [u8; 16],
    source: [u8; 32],
}
impl source_reservation_seal::Sealed for CakeMachineEntry {}
impl ScopeCreatureReservationProof for CakeMachineEntry {
    fn scope(&self) -> (RuntimeScopeRefV1, ScopeOwnershipGeneration) {
        (self.scope, self.generation)
    }
    fn event_identity(&self) -> [u8; 16] {
        self.event
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        self.source
    }
    fn affected_creatures(&self) -> &[ExactActorRef] {
        &self.actors
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime.binding().runtime_scope() == self.scope
            && runtime.binding().scope_generation() == self.generation
            && runtime.matches_live_creature_identity(self.actor(), CREATURE.as_bytes())
            && runtime.read_actor_position(self.actor()) == Ok(self.position)
            && runtime.raid_identity_for_actor(self.actor()) == Ok(Some(self.identity))
    }
}
impl CakeMachineEntry {
    /// Reconstruct only immutable binding evidence. The physical step owner
    /// independently checks machine Item8886 before installing the reservation.
    pub fn from_current(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> Option<Self> {
        if !runtime.matches_live_creature_identity(actor, CREATURE.as_bytes()) {
            return None;
        }
        let identity = runtime.raid_identity_for_actor(actor).ok()??;
        let position = runtime.read_actor_position(actor).ok()?;
        let at = position.position();
        let coords = [
            u16::try_from(at.x).ok()?,
            u16::try_from(at.y).ok()?,
            u16::try_from(at.floor).ok()?,
        ];
        if !crate::durability::raid_runner::cake::is_machine_position(coords) {
            return None;
        }
        let binding = runtime.binding();
        let mut digest = Sha256::new();
        digest.update(b"oteryn/cake-step/201189f/v1");
        digest.update(identity.firing_id);
        for value in [identity.wave, identity.entry, identity.index] {
            digest.update(value.to_be_bytes());
        }
        digest.update(binding.world_id().as_bytes());
        digest.update(binding.channel_id()?.as_bytes());
        digest.update(binding.scope_generation().get().to_be_bytes());
        digest.update(actor.actor_local_id().to_be_bytes());
        digest.update(actor.actor_local_generation().to_be_bytes());
        digest.update(position.revision().to_be_bytes());
        for coordinate in coords {
            digest.update(coordinate.to_be_bytes());
        }
        let source: [u8; 32] = digest.finalize().into();
        let event = source[..16].try_into().ok()?;
        Some(Self {
            scope: binding.runtime_scope(),
            generation: binding.scope_generation(),
            identity,
            actors: [actor],
            position,
            coordinates: coords,
            event,
            source,
        })
    }
    pub fn identity(&self) -> RaidSpawnIdentity {
        self.identity
    }
    pub fn actor(&self) -> ExactActorRef {
        self.actors[0]
    }
    pub fn position(&self) -> crate::foundation::MovementLocalPosition {
        self.position.position()
    }
    pub fn coordinates(&self) -> [u16; 3] {
        self.coordinates
    }
}

pub fn reserve_machine_step(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    machine_present: bool,
) {
    if !machine_present {
        return;
    }
    let Some(entry) = CakeMachineEntry::from_current(runtime, actor) else {
        return;
    };
    let Ok(mut reservation) = runtime.prepare_scope_creature_reservation(&entry) else {
        return;
    };
    let _ = runtime.reserve_scope_creatures(&mut reservation, &entry);
    // Dropping the local handle never releases pending_source. The raid owner
    // resumes this exact frozen slot after its placement ACK and durable lure.
}
