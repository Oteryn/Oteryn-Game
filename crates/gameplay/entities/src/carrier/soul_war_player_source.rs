//! Current-pass player facts; these enrich an independently fenced HP owner.
use super::*;
use oteryn_gameplay_content::content::spell_data::Vocation;

pub mod source_proof_seal {
    pub trait Sealed {}
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulWarPlayerSourceOrigin {
    Combat,
    Condition,
    Reflection,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulWarRealmMonster {
    Brachiodemon,
    ManyFaces,
    BonySeaDevil,
    CloakOfTerror,
    BranchyCrawler,
    DreadfulHarvester,
}
impl SoulWarRealmMonster {
    pub fn creature_key(self) -> &'static str {
        match self {
            Self::Brachiodemon => "oteryn:creature.brachiodemon",
            Self::ManyFaces => "oteryn:creature.many_faces",
            Self::BonySeaDevil => "oteryn:creature.bony_sea_devil",
            Self::CloakOfTerror => "oteryn:creature.cloak_of_terror",
            Self::BranchyCrawler => "oteryn:creature.branchy_crawler",
            Self::DreadfulHarvester => "oteryn:creature.dreadful_harvester",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulWarPlayerSourceFacts {
    pub vocation: Vocation,
    pub taints: u8,
    pub realm: Option<SoulWarRealmMonster>,
    pub before_revision: u64,
    pub origin: SoulWarPlayerSourceOrigin,
}
/// Implemented by the real private current Quest reader composition, not by a
/// wire request. Detached data never replaces current actor/namespace authority.
pub trait CurrentSoulWarPlayerSourceProof: source_proof_seal::Sealed {
    fn current_facts(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        command: Option<CommandRef>,
        now_us: u64,
    ) -> Option<SoulWarPlayerSourceFacts>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CurrentPlayerSource {
    pub(super) actor: ExactActorRef,
    pub(super) session: GameSessionId,
    pub(super) character: [u8; 16],
    pub(super) lease: u64,
    pub(super) command: Option<CommandRef>,
    pub(super) now_us: u64,
    pub(super) scope: RuntimeScopeRefV1,
    pub(super) generation: u64,
    pub(super) server: [u8; 32],
    pub(super) client: [u8; 32],
    pub(super) frame: [u8; 32],
    pub(super) map: [u8; 32],
    pub(super) position: MovementLocalPosition,
    pub(super) facts: SoulWarPlayerSourceFacts,
}
impl ChannelRuntimeV1 {
    pub fn install_current_soul_war_player_source(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        command: Option<CommandRef>,
        now_us: u64,
        proof: &impl CurrentSoulWarPlayerSourceProof,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let control = self.player_control_facts(actor, session)?;
        if command.is_some_and(|command| command.game_session_id() != session) {
            return Err(CarrierError::PlanConflict);
        }
        let facts = proof
            .current_facts(self, actor, session, command, now_us)
            .ok_or(CarrierError::PlanConflict)?;
        // Existing condition effects observe retained visible avatars; these
        // facts are not a fresh command or a grant to create a new attack.
        if facts.origin == SoulWarPlayerSourceOrigin::Combat && control.control_loss.is_some() {
            return Err(CarrierError::PlanConflict);
        }
        if facts.taints > 5
            || (facts.origin == SoulWarPlayerSourceOrigin::Combat && command.is_none())
        {
            return Err(CarrierError::InvalidCommitBinding);
        }
        let lease = self.bound_attacker_lease(actor, session)?;
        let pin = self.content_pin();
        let current = CurrentPlayerSource {
            actor,
            session,
            character: *lease.character_id().as_bytes(),
            lease: lease.generation(),
            command,
            now_us,
            scope: self.binding().runtime_scope(),
            generation: self.binding().scope_generation().get(),
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: self.map_revision_digest(),
            position: self.read_actor_position(actor)?.position(),
            facts,
        };
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::Occupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.soul_war_player_source = Some(Arc::new(current));
        Ok(())
    }
    pub fn current_soul_war_player_source(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command: Option<CommandRef>,
        now_us: u64,
    ) -> Result<Option<SoulWarPlayerSourceFacts>, CarrierError> {
        self.owner_fence()?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        let Slot::Occupied { spell_combat, .. } = &self.carrier.slots[index] else {
            return Ok(None);
        };
        let Some(current) = &spell_combat.soul_war_player_source else {
            return Ok(None);
        };
        let pin = self.content_pin();
        let lease = self.bound_attacker_lease(actor, session)?;
        if current.actor != actor
            || current.session != session
            || current.command != command
            || current.now_us != now_us
            || current.scope != self.binding().runtime_scope()
            || current.generation != self.binding().scope_generation().get()
            || current.server != pin.server_artifact_digest()
            || current.client != pin.client_artifact_digest()
            || current.frame != pin.frame_binding_digest()
            || current.map != self.map_revision_digest()
            || current.character != *lease.character_id().as_bytes()
            || current.lease != lease.generation()
            || (current.facts.origin == SoulWarPlayerSourceOrigin::Combat
                && self
                    .player_control_facts(actor, session)?
                    .control_loss
                    .is_some())
            || self.read_actor_position(actor)?.position() != current.position
        {
            return Ok(None);
        }
        Ok(Some(current.facts))
    }
}
