//! Read-only cache of a genuinely observed World party, stored on the actual actor slot.
//! The authoritative membership is PostgreSQL; this cache never grants consent, membership,
//! caster legality or current authority. None means unavailable, never an invented solo party.
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub(crate) enum PartyAction {
    Invite { target: [u8; 16] },
    Accept { party: [u8; 16] },
    Decline { party: [u8; 16] },
    Revoke { target: [u8; 16] },
    Leave,
    TransferLeader { target: [u8; 16] },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PartyCommand {
    pub(crate) actor: ExactActorRef,
    pub(crate) session: GameSessionId,
    pub(crate) command: CommandRef,
    pub(crate) action: PartyAction,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PartyReceipt {
    pub(crate) applied: bool,
    pub(crate) party_id: Option<[u8; 16]>,
    pub(crate) revision: Option<u64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartyPresence {
    Channel,
    Hidden,
    Offline,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartyMemberSnapshot {
    pub(crate) character: [u8; 16],
    pub(crate) invitation_order: u64,
    pub(crate) presence_revision: u64,
    pub(crate) presence: PartyPresence,
    pub(crate) channel: Option<[u8; 16]>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartySnapshot {
    pub(crate) world: [u8; 16],
    pub(crate) character: [u8; 16],
    pub(crate) party_id: Option<[u8; 16]>,
    pub(crate) revision: Option<u64>,
    pub(crate) leader: Option<[u8; 16]>,
    pub(crate) members: Vec<PartyMemberSnapshot>,
    pub(crate) observed_database_micros: i64,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PartyActorState {
    projection: Option<PartySnapshot>,
}
impl ChannelRuntimeV1 {
    /// A presentation/cache update only. Spell consumers use a fresh same-TX strong World read.
    pub(crate) fn project_world_party(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        projection: PartySnapshot,
    ) -> Result<(), CarrierError> {
        self.assert_actor_spell_unreserved(actor)?;
        if self
            .player_control_facts(actor, session)?
            .control_loss
            .is_some()
            || projection.world != *self.binding.world_id.as_bytes()
            || projection.members.len() > 50
            || (projection.party_id.is_none()
                && (!projection.members.is_empty()
                    || projection.revision.is_some()
                    || projection.leader.is_some()))
        {
            return Err(CarrierError::PlanConflict);
        }
        let index = self
            .carrier
            .player_slot_index(&self.continuity, actor.0, session)?;
        let Slot::Occupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::PlanConflict);
        };
        spell_combat.party.projection = Some(projection);
        Ok(())
    }
    pub(crate) fn cached_world_party(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Option<&PartySnapshot>, CarrierError> {
        let index = self
            .carrier
            .player_slot_index(&self.continuity, actor.0, session)?;
        let Slot::Occupied { spell_combat, .. } = &self.carrier.slots[index] else {
            return Err(CarrierError::PlanConflict);
        };
        Ok(spell_combat.party.projection.as_ref())
    }
}
