//! M51 truthful one-item pit reset cause. Crystal201189f
//! lib/quests/svargrond_arena.lua:217-225. These are audit facts, never authority:
//! the retirement writer must independently prove current pit custody and stack.
use super::*;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemArenaResetV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub channel_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub game_session_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub run_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "6")]
    pub item_instance_id: Vec<u8>,
    #[prost(uint32, tag = "7")]
    pub rank: u32,
    #[prost(uint32, tag = "8")]
    pub pit: u32,
    #[prost(uint64, tag = "9")]
    pub deadline_unix_ms: u64,
}

pub(super) fn check(value: &OneItemDecayRetireV1) -> Result<DecayRetireShape, AuditError> {
    let cause = value.arena_reset.as_ref().ok_or(AuditError::InvalidInput)?;
    let before = value.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let ground = value.ground.as_ref().ok_or(AuditError::InvalidInput)?;
    check_state(before)?;
    check_state(after)?;
    mint::check_key_number(before)?;
    check_ground(ground)?;
    for identity in [
        &cause.world_id,
        &cause.channel_id,
        &cause.character_id,
        &cause.game_session_id,
        &cause.run_id,
        &cause.item_instance_id,
    ] {
        check_uuid_v7(identity)?;
    }
    let pit = u8::try_from(cause.pit)
        .ok()
        .and_then(oteryn_gameplay_quests::quest::barbarian_arena::Pit::new)
        .ok_or(AuditError::InvalidInput)?;
    let at = super::super::spell_items_abi::decode_ground_cell(&ground.spatial_position)
        .map_err(|_| AuditError::InvalidInput)?;
    if value.cause.is_some()
        || value.world_reset.is_some()
        || value.activity_retirement.is_some()
        || value.corpse_entry.is_some()
        || value.ground_container_entry.is_some()
        || before.item_instance_id != after.item_instance_id
        || before.world_id != after.world_id
        || before.definition != after.definition
        || before.key_number != after.key_number
        || before.lifecycle != ITEM_LIFECYCLE_LIVE
        || before.quantity == 0
        || after.lifecycle != ITEM_LIFECYCLE_RETIRED
        || after.quantity != 0
        || cause.world_id != before.world_id
        || ground.world_id != before.world_id
        || cause.item_instance_id != before.item_instance_id
        || ground.channel_id != cause.channel_id
        || ground.instance_id.is_some()
        || value.runtime_scope_ownership_generation == 0
        || ground.runtime_scope_ownership_generation != value.runtime_scope_ownership_generation
        || !(1..=3).contains(&cause.rank)
        || cause.deadline_unix_ms == 0
        || cause.deadline_unix_ms > i64::MAX as u64
        || !pit.layout().contains_reset([at.x, at.y, at.z])
    {
        return Err(AuditError::InvalidInput);
    }
    DECAY_RETIRE_USAGE.check()?;
    Ok(DecayRetireShape::Ground)
}

#[cfg(test)]
#[path = "arena_reset_tests.rs"]
mod tests;
