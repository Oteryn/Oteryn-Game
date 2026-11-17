//! Read-only public NPC markers of the current runtime context; excludes private hirelings.
use super::*;
impl ChannelRuntimeV1 {
    pub fn npc_atlas_positions(&self) -> impl Iterator<Item = (&str, LocalPosition)> {
        self.carrier.occupied.iter().filter_map(|index| {
            match &self.carrier.slots[*index as usize] {
                Slot::NpcOccupied {
                    key,
                    hireling: None,
                    position,
                    ..
                } if position.context == self.pinned_position_context() => {
                    Some((key.as_ref(), position.position))
                }
                _ => None,
            }
        })
    }
}
