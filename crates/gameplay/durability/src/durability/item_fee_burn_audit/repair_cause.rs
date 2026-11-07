//! The repair and fee are one combined native item transaction event.
use super::*;
use crate::durability::item_timed_state_audit::OneItemTimedRowV1;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemTimedRepairV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub occurrence: Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "3")]
    pub after: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "4")]
    pub row_before: Option<OneItemTimedRowV1>,
    #[prost(message, optional, tag = "5")]
    pub row_after: Option<OneItemTimedRowV1>,
    #[prost(uint64, tag = "6")]
    pub placement_ordinal: u64,
}

pub(in crate::durability) fn check(value: &OneItemTimedRepairV1) -> Result<(), AuditError> {
    check_uuid_v7(&value.occurrence)?;
    let (Some(before), Some(after), Some(row_before), Some(row_after)) = (
        &value.before,
        &value.after,
        &value.row_before,
        &value.row_after,
    ) else {
        return Err(AuditError::InvalidInput);
    };
    check_uuid_v7(&before.item_instance_id)?;
    check_uuid_v7(&before.world_id)?;
    crate::durability::item_mint_audit::check_definition(before.definition.as_ref())?;
    crate::durability::item_mint_audit::check_definition(after.definition.as_ref())?;
    let (Some(from), Some(to)) = (&before.definition, &after.definition) else {
        return Err(AuditError::InvalidInput);
    };
    if before.item_instance_id != after.item_instance_id
        || before.world_id != after.world_id
        || before.quantity != 1
        || after.quantity != 1
        || before.lifecycle != 1
        || after.lifecycle != 1
        || before.key_number != 0
        || after.key_number != 0
        || value.placement_ordinal == 0
        || from.family != "Item"
        || to.family != "Item"
        || from.production_key != "oteryn:item.tibia.i6530"
        || to.production_key != "oteryn:item.tibia.i6529"
        || from.revision_ref.is_empty()
        || to.revision_ref.is_empty()
        || row_before.revision.checked_add(1) != Some(row_after.revision)
        || row_before.charges.is_some()
        || row_before.remaining_ms.is_some()
        || row_after.charges.is_some()
        || row_after.remaining_ms != Some(14_400_000)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

pub(super) fn check_usage(usage: &TransactionResourceUsage) -> Result<(), AuditError> {
    if usage.transform_lines != 1 || usage.container_expansion != 0 {
        return Err(AuditError::InvalidInput);
    }
    if usage.touched_item_instances > 22
        || usage.participants > 22
        || usage.effect_work_units > 68
        || usage.location_custody_lines > 21
        || usage.events > 1
        || usage.value_lines > 1
    {
        return Err(AuditError::CapacityExceeded);
    }
    Ok(())
}
