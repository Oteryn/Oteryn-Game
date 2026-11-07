//! Existing world and fenced NPC owners authorize the same atomic Item debit.
use super::{CurrentCharacterGameplayFence, DurabilityError, SpellLanePermit};
use crate::durability::quest_state::npc_program::NpcItemPermit;

pub(super) enum InputAuthority<'a> {
    Spell(&'a SpellLanePermit),
    Npc(&'a NpcItemPermit),
}

impl InputAuthority<'_> {
    pub(super) fn check(
        &self,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<(), DurabilityError> {
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        match self {
            Self::Spell(permit) => permit.check_channel(world_id, channel_id)?,
            Self::Npc(permit) => {
                // This opaque permit is constructed only by the sequenced NPC owner
                // after the current physical equipment fence has been admitted.
                let _checked_npc_owner = permit;
            }
        }
        Ok(())
    }
}
