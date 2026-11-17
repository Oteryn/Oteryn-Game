//! Native consumption of elder_bloodjaw.lua's boss-presence despawn.
use super::elder_bloodjaw_think::{
    BOSS_PRESENCE_RANGE_TILES, ELDER_BLOODJAW_KEY, is_rotten_blood_boss,
};
use oteryn_gameplay_entities::{CarrierError, ChannelRuntimeV1, ExactActorRef};

/// Population repairs belong to the room, not a parent's random-wave list.
pub fn remove_orphan(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    content: [u8; 32],
) -> Result<bool, CarrierError> {
    super::native_mushroom::remove_orphan_with_presence(
        runtime,
        actor,
        content,
        ELDER_BLOODJAW_KEY,
        is_rotten_blood_boss,
        BOSS_PRESENCE_RANGE_TILES,
    )
}
