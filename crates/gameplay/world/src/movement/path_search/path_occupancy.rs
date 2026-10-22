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
