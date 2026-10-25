//! The shrine apply or clear and its fee are one combined native item transaction event
//! (PARITY-IMBUE-FORGE-WIRE-1, migration 0196).
use super::*;
use crate::durability::item_mint_audit::check_content_key;

pub const IMBUEMENT_ACTION_APPLY: u32 = 1;
pub const IMBUEMENT_ACTION_CLEAR: u32 = 2;
/// Canary `imbuements.xml` base prices by tier and `removecost`.
pub const IMBUEMENT_APPLY_GOLD: [u64; 3] = [5_000, 30_000, 200_000];
pub const IMBUEMENT_CLEAR_GOLD: u64 = 15_000;
/// Coin inputs of an imbuement fee: with the item, three materials and the change it stays
/// within the fee rows `FEE_RL01`/`RL02`/`RL06` and the `FEE_RL07` payload.
pub const IMBUEMENT_FEE_INPUTS_MAX: usize = 15;
pub const IMBUEMENT_MATERIALS_MAX: usize = 3;

/// One consumed material: a plain stack in a direct entry of the same backpack.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemImbuementMaterialV1 {
    #[prost(message, optional, tag = "1")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub after: Option<OneItemStateV1>,
    #[prost(uint64, tag = "3")]
    pub placement_ordinal: u64,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemImbuementV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub occurrence: Vec<u8>,
    /// 1 apply, 2 clear.
    #[prost(uint32, tag = "2")]
    pub action: u32,
    /// The imbued item, unchanged by the command.
    #[prost(message, optional, tag = "3")]
    pub item: Option<OneItemStateV1>,
    #[prost(uint64, tag = "4")]
    pub placement_ordinal: u64,
    #[prost(uint32, tag = "5")]
    pub slot: u32,
    #[prost(uint32, tag = "6")]
    pub category: u32,
    #[prost(string, tag = "7")]
    pub imbuement_key: String,
    #[prost(string, tag = "8")]
    pub imbuement_revision: String,
    #[prost(uint32, tag = "9")]
    pub tier: u32,
    #[prost(message, repeated, tag = "10")]
    pub materials: Vec<OneItemImbuementMaterialV1>,
}

impl OneItemImbuementV1 {
    /// The fee this command pays.
    pub fn fee_gold_units(&self) -> Option<u64> {
        match self.action {
            IMBUEMENT_ACTION_APPLY => usize::try_from(self.tier)
                .ok()
                .and_then(|tier| tier.checked_sub(1))
                .and_then(|index| IMBUEMENT_APPLY_GOLD.get(index).copied()),
            IMBUEMENT_ACTION_CLEAR => Some(IMBUEMENT_CLEAR_GOLD),
            _ => None,
        }
    }

    /// Every item instance the command touches besides its coins.
    pub fn item_ids(&self) -> impl Iterator<Item = &[u8]> {
        self.item
            .iter()
            .map(|item| item.item_instance_id.as_slice())
            .chain(self.materials.iter().filter_map(|material| {
                material
                    .before
                    .as_ref()
                    .map(|before| before.item_instance_id.as_slice())
            }))
    }
}

fn plain_item(state: &OneItemStateV1, world: &[u8]) -> Result<(), AuditError> {
    check_uuid_v7(&state.item_instance_id)?;
    check_definition(state.definition.as_ref())?;
    if state.world_id != world
        || state.key_number != 0
        || state
            .definition
            .as_ref()
            .is_none_or(|definition| definition.family != "Item")
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

pub(in crate::durability) fn check(value: &OneItemImbuementV1) -> Result<(), AuditError> {
    check_uuid_v7(&value.occurrence)?;
    check_content_key(&value.imbuement_key)?;
    check_content_key(&value.imbuement_revision)?;
    let item = value.item.as_ref().ok_or(AuditError::InvalidInput)?;
    check_uuid_v7(&item.world_id)?;
    plain_item(item, &item.world_id)?;
    let materials = match value.action {
        IMBUEMENT_ACTION_APPLY => 1..=IMBUEMENT_MATERIALS_MAX,
        IMBUEMENT_ACTION_CLEAR => 0..=0,
        _ => return Err(AuditError::InvalidInput),
    };
    if item.quantity != 1
        || item.lifecycle != ITEM_LIFECYCLE_LIVE
        || value.placement_ordinal == 0
        || value.slot >= 3
        || value.category >= 20
        || !(1..=3).contains(&value.tier)
        || !materials.contains(&value.materials.len())
    {
        return Err(AuditError::InvalidInput);
    }
    for (index, material) in value.materials.iter().enumerate() {
        let (Some(before), Some(after)) = (&material.before, &material.after) else {
            return Err(AuditError::InvalidInput);
        };
        plain_item(before, &item.world_id)?;
        let whole = after.quantity == 0 && after.lifecycle == ITEM_LIFECYCLE_RETIRED;
        let partial = after.quantity > 0 && after.lifecycle == ITEM_LIFECYCLE_LIVE;
        if before.item_instance_id != after.item_instance_id
            || before.item_instance_id == item.item_instance_id
            || before.world_id != after.world_id
            || before.definition != after.definition
            || before.key_number != after.key_number
            || before.lifecycle != ITEM_LIFECYCLE_LIVE
            || after.quantity >= before.quantity
            || !(whole || partial)
            || material.placement_ordinal == 0
            || material.placement_ordinal == value.placement_ordinal
            || value.materials[..index].iter().any(|other| {
                other.placement_ordinal == material.placement_ordinal
                    || other.before.as_ref().map(|b| &b.item_instance_id)
                        == Some(&before.item_instance_id)
            })
        {
            return Err(AuditError::InvalidInput);
        }
    }
    Ok(())
}

/// The imbuement's item and materials beside the coin lines: the item is one participant whose
/// work is its one imbuement row change; a material is a burn line of the same weight.
pub(super) fn usage(value: &OneItemImbuementV1) -> TransactionResourceUsage {
    let count = value.materials.len() as u64;
    let whole = value
        .materials
        .iter()
        .filter(|material| material.after.as_ref().is_some_and(|after| after.quantity == 0))
        .count() as u64;
    TransactionResourceUsage {
        touched_item_instances: 1 + count,
        location_custody_lines: whole,
        value_lines: 0,
        transform_lines: 0,
        container_expansion: 0,
        participants: 1 + count,
        effect_work_units: 2 + 3 * whole + 2 * (count - whole),
        events: 0,
    }
}
