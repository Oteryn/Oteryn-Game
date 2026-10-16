//! Read-only cache of a genuinely observed World party, stored on the actual actor slot.
//! The authoritative membership is PostgreSQL; this cache never grants consent, membership,
//! caster legality or current authority. None means unavailable, never an invented solo party.
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub(crate) enum PartyAction {
    Invite {
        target: [u8; 16],
    },
    Accept {
        party: [u8; 16],
    },
    Decline {
        party: [u8; 16],
    },
    Revoke {
        target: [u8; 16],
    },
    Leave,
    TransferLeader {
        target: [u8; 16],
    },
    SetSharedExperience {
        enabled: bool,
    },
    /// Server operations; an untrusted wire child resolves its own actor/opaque handles.
    Block {
        target: [u8; 16],
    },
    Unblock {
        target: [u8; 16],
    },
    SetPartyInvites {
        anyone: bool,
    },
    SetChannelVisibility {
        hidden: bool,
    },
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
    /// Observed durable preference only; actual XP eligibility remains its owner's decision.
    pub(crate) shared_xp_enabled: bool,
    pub(crate) members: Vec<PartyMemberSnapshot>,
    pub(crate) observed_database_micros: i64,
}
impl PartySnapshot {
    /// Shape check only. Neither this value nor the actor cache grants authority/freshness.
    pub(crate) fn is_well_formed(&self) -> bool {
        if self.party_id.is_none() {
            return self.revision.is_none()
                && self.leader.is_none()
                && !self.shared_xp_enabled
                && self.members.is_empty();
        }
        let members = &self.members;
        let unique: std::collections::BTreeSet<_> =
            members.iter().map(|member| member.character).collect();
        let sequences: std::collections::BTreeSet<_> = members
            .iter()
            .map(|member| member.invitation_order)
            .collect();
        self.revision.is_some_and(|revision| revision != 0)
            && (1..=50).contains(&members.len())
            && unique.len() == members.len()
            && sequences.len() == members.len()
            && unique.contains(&self.character)
            && self.leader.is_some_and(|leader| unique.contains(&leader))
            && members
                .windows(2)
                .all(|pair| pair[0].invitation_order < pair[1].invitation_order)
            && members.iter().all(|member| {
                member.invitation_order != 0
                    && member.presence_revision != 0
                    && match member.presence {
                        PartyPresence::Channel => member.channel.is_some(),
                        PartyPresence::Hidden | PartyPresence::Offline => member.channel.is_none(),
                    }
            })
    }
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
            || !projection.is_well_formed()
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
