//! ADR-0021 §4.7: additive reset cause and Ground-container location grammar.
use super::*;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemWorldResetV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub world_id: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub reset_epoch: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub item_instance_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub target_digest: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub retirement_binding: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemGroundContainerEntryV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub parent_item_instance_id: Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub placement_ordinal: u64,
    #[prost(message, optional, tag = "3")]
    pub parent_ground: Option<OneItemGroundV1>,
    /// When present, Ground belongs to this death-tree root; `parent` is the actual edge parent.
    /// Absent for legacy direct Ground-container entries, preserving their encoded bytes.
    #[prost(bytes = "vec", optional, tag = "4")]
    pub tree_root_item_instance_id: Option<Vec<u8>>,
}

pub(super) fn check_entry(
    entry: &OneItemGroundContainerEntryV1,
    item: &[u8],
) -> Result<(), AuditError> {
    check_uuid_v7(&entry.parent_item_instance_id)?;
    if entry.parent_item_instance_id == item || entry.placement_ordinal == 0 {
        return Err(AuditError::InvalidInput);
    }
    if let Some(root) = &entry.tree_root_item_instance_id {
        check_uuid_v7(root)?;
        if root == item {
            return Err(AuditError::InvalidInput);
        }
    } else if entry.placement_ordinal > mint::GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

pub(super) fn check(value: &OneItemDecayRetireV1) -> Result<DecayRetireShape, AuditError> {
    let cause = value.world_reset.as_ref().ok_or(AuditError::InvalidInput)?;
    let before = value.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    check_state(before)?;
    check_state(after)?;
    mint::check_key_number(before)?;
    check_uuid_v7(&cause.world_id)?;
    check_uuid_v7(&cause.item_instance_id)?;
    let ground = scope_ground(value)?;
    check_ground(ground)?;
    if value.activity_retirement.is_some()
        || ground.instance_id.is_some()
        || value.cause.is_some()
        || value.corpse_entry.is_some()
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
        || cause.reset_epoch == u64::MAX
        || cause.target_digest.len() != 32
        || cause.target_digest.iter().all(|byte| *byte == 0)
        || value.runtime_scope_ownership_generation == 0
    {
        return Err(AuditError::InvalidInput);
    }
    let binding = Sha256::new()
        .chain_update(b"oteryn:world-reset-retirement:v1")
        .chain_update(&cause.target_digest)
        .finalize();
    if cause.retirement_binding.as_slice() != binding.as_slice() {
        return Err(AuditError::InvalidInput);
    }
    DECAY_RETIRE_USAGE.check()?;
    if let Some(entry) = &value.ground_container_entry {
        check_entry(entry, &before.item_instance_id)?;
        Ok(DecayRetireShape::GroundContainerEntry)
    } else {
        Ok(DecayRetireShape::Ground)
    }
}

#[cfg(test)]
#[path = "world_reset_tests.rs"]
mod tests;
