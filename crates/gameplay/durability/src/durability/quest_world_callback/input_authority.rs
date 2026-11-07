//! Retain each existing owner's fenced authority when sharing the bounded item writer.
use super::{CurrentCharacterGameplayFence, DurabilityError, SpellLanePermit};
use crate::durability::quest_state::npc_program::NpcItemPermit;

pub(super) enum InputAuthority<'a> {
    Spell(&'a SpellLanePermit),
    Npc(&'a NpcItemPermit),
}
impl InputAuthority<'_> {
    pub(super) fn check(self, fence: CurrentCharacterGameplayFence) -> Result<(), DurabilityError> {
        match self {
            Self::Spell(permit) => {
                let crate::foundation::RuntimeScopeRefV1::Channel {
                    world_id,
                    channel_id,
                } = fence.runtime_scope
                else {
                    return Err(DurabilityError::InvalidStoredState);
                };
                permit.check_channel(world_id, channel_id)
            }
            // The NPC owner constructs this sealed permit only after checking the
            // current equipment fence in the same transaction (npc_program/items.rs).
            // It does not run in a Spell lane or bypass the NPC's fence admission.
            Self::Npc(permit) => {
                let _ = permit;
                Ok(())
            }
        }
    }
}
