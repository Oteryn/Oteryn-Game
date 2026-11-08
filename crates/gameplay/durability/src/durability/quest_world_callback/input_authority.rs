//! The carried writer is shared by two already fenced, separately typed owners.
use super::*;
use crate::durability::quest_state::npc_program::NpcItemPermit;

pub(super) enum InputPermit<'a> {
    Spell(&'a SpellLanePermit),
    Npc(&'a NpcItemPermit),
}

impl<'a> From<&'a SpellLanePermit> for InputPermit<'a> {
    fn from(permit: &'a SpellLanePermit) -> Self {
        Self::Spell(permit)
    }
}

impl<'a> From<&'a NpcItemPermit> for InputPermit<'a> {
    fn from(permit: &'a NpcItemPermit) -> Self {
        Self::Npc(permit)
    }
}

impl InputPermit<'_> {
    pub(super) fn check(self, fence: CurrentCharacterGameplayFence) -> Result<(), DurabilityError> {
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        match self {
            Self::Spell(permit) => permit.check_channel(world_id, channel_id),
            // Only the NPC programme's fenced value participant constructs this brand.
            Self::Npc(_permit) => Ok(()),
        }
    }
}
