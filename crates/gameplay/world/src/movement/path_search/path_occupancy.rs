//! Shared read-only occupancy evidence for path costs, collected once while the owner is held.
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};

pub fn standing_positions(
    runtime: &ChannelRuntimeV1,
) -> Vec<(ExactActorRef, MovementLocalPosition)> {
    let visible = runtime.visible_entities();
    visible
        .creatures
        .iter()
        .chain(&visible.npcs)
        .chain(&visible.players)
        .map(|other| (other.actor, other.position))
        .collect()
}

/// One occupancy cut under the held runtime lock, shared by all tiles of a search drain.
pub struct PathOccupancy {
    counts: std::collections::BTreeMap<(i32, i32, i16), usize>,
}
impl PathOccupancy {
    pub fn current(runtime: &ChannelRuntimeV1) -> Self {
        let mut counts = std::collections::BTreeMap::new();
        for (_, at) in standing_positions(runtime) {
            *counts.entry((at.x, at.y, at.floor)).or_default() += 1;
        }
        Self { counts }
    }
    pub fn extra_cost(
        &self,
        at: MovementLocalPosition,
        own_position: MovementLocalPosition,
    ) -> u32 {
        if self
            .counts
            .get(&(at.x, at.y, at.floor))
            .copied()
            .unwrap_or(0)
            > usize::from(at == own_position)
        {
            super::CREATURE_COST
        } else {
            0
        }
    }
}
