//! Opaque native kill facts. Only the current physical owner can construct this
//! proof from its retained lethal projection and actual dead creature policy.
use super::*;

#[derive(Debug, Clone)]
pub struct NativeCreditedKillProof {
    death: CreatureDeathOccurrenceKey,
    creature: String,
    position: MovementLocalPosition,
    principal: TopDamageContributor,
    occurrence: [u8; 16],
    map: [u8; 32],
    content: [u8; 32],
    frame: [u8; 32],
}
impl NativeCreditedKillProof {
    pub const fn death(&self) -> CreatureDeathOccurrenceKey {
        self.death
    }
    pub fn creature(&self) -> &str {
        &self.creature
    }
    pub const fn position(&self) -> MovementLocalPosition {
        self.position
    }
    pub const fn principal(&self) -> TopDamageContributor {
        self.principal
    }
    pub const fn occurrence(&self) -> [u8; 16] {
        self.occurrence
    }
    pub const fn map_digest(&self) -> [u8; 32] {
        self.map
    }
    pub const fn content_digest(&self) -> [u8; 32] {
        self.content
    }
    pub const fn frame_digest(&self) -> [u8; 32] {
        self.frame
    }
}

impl ChannelRuntimeV1 {
    pub fn native_credited_kill_proof(
        &mut self,
        target: ExactActorRef,
    ) -> Result<NativeCreditedKillProof, CarrierError> {
        let snapshot = self.companion_snapshot_including_dead(target)?;
        if snapshot.health != 0
            || snapshot.state.master.is_some()
            || snapshot.state.policy.is_familiar
        {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        let creature = snapshot.state.policy.definition_key.clone();
        let mut owner = self.borrow_combat_death();
        let (death, position) = owner.projected_death(target)?;
        let principal = owner
            .top_damage_contributor(target)?
            .present()
            .ok_or(CarrierError::CommittedLethalUnavailable)?;
        let (occurrence, _) = owner.reward_occurrence(target, *principal.character.as_bytes())?;
        Ok(NativeCreditedKillProof {
            death,
            creature,
            position,
            principal,
            occurrence,
            map: self.content_pin().map_revision_digest(),
            content: self.content_pin().server_artifact_digest(),
            frame: self.content_pin().frame_binding_digest(),
        })
    }
}
