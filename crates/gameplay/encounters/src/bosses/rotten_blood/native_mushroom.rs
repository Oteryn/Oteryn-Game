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
    let stamp = runtime.issue_owner_work()?;
    if content == [0; 32] || content != runtime.content_pin().server_artifact_digest() {
        return Err(CarrierError::CreatureTargetMismatch);
    }
    let mushroom = runtime.companion_snapshot(actor)?;
    if mushroom.state.policy.definition_key != MUSHROOM_KEY
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
    for (candidate, position, _) in runtime.positioned_melee_census()? {
        let position = position.position();
        if !own.in_range(
            RoomPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
            BOSS_PRESENCE_RANGE_TILES,
        ) {
            continue;
        }
        if runtime.companion_snapshot(candidate).is_ok_and(|snapshot| {
            snapshot.health > 0 && is_mushroom_boss(&snapshot.state.policy.definition_key)
        }) {
            return Ok(false);
        }
    }
    runtime.remove_current_native_encounter_subject(stamp, actor, MUSHROOM_KEY, content)?;
    Ok(true)
}
