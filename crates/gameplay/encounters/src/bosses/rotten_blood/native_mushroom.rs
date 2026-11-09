//! Current-owner consumption of the source Mushroom boss-presence rule.
use super::RoomPosition;
use super::mushroom_think::{BOSS_PRESENCE_RANGE_TILES, MUSHROOM_KEY, is_mushroom_boss};
use oteryn_gameplay_entities::{CarrierError, ChannelRuntimeV1, ExactActorRef};

/// Administrative source despawn; this does not award death consequences.
/// Presence comes from current native identities, never display names or cached waves.
pub fn remove_orphan(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    content: [u8; 32],
) -> Result<bool, CarrierError> {
    remove_orphan_with_presence(
        runtime,
        actor,
        content,
        MUSHROOM_KEY,
        is_mushroom_boss,
        BOSS_PRESENCE_RANGE_TILES,
    )
}

pub(super) fn remove_orphan_with_presence(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    content: [u8; 32],
    expected_key: &str,
    is_boss: fn(&str) -> bool,
    range: u16,
) -> Result<bool, CarrierError> {
    let stamp = runtime.issue_owner_work()?;
    if content == [0; 32] || content != runtime.content_pin().server_artifact_digest() {
        return Err(CarrierError::CreatureTargetMismatch);
    }
    let mushroom = runtime.companion_snapshot(actor)?;
    if mushroom.state.policy.definition_key != expected_key
        || mushroom.state.master.is_some()
        || runtime.native_summon_master(actor).is_some()
    {
        return Err(CarrierError::CreatureTargetMismatch);
    }
    let own = RoomPosition {
        x: mushroom.position.x,
        y: mushroom.position.y,
        floor: mushroom.position.floor,
    };
    for (candidate, position, session) in runtime.positioned_melee_census()? {
        if session.is_some() {
            continue;
        }
        let position = position.position();
        if !own.in_range(
            RoomPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
            range,
        ) {
            continue;
        }
        // A missing native policy withholds the absence proof. Players were
        // excluded above; every remaining census row is a living creature.
        let snapshot = runtime.companion_snapshot(candidate)?;
        if snapshot.health > 0 && is_boss(&snapshot.state.policy.definition_key) {
            return Ok(false);
        }
    }
    runtime.remove_current_native_encounter_subject(stamp, actor, expected_key, content)?;
    Ok(true)
}
