//! A closed additive variant for multi-output NPC purchases. Legacy checks remain unchanged.
use super::super::item_mint_audit::{OneItemGroundV1, OneItemTypedDefinitionRevisionV1};
use super::*;
use oteryn_gameplay_economy::npc_purchase::{SHOPPING_BAG_PRICE, SHOPPING_BAG_SLOTS};

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemNpcBulkPurchaseV1 {
    #[prost(message, optional, tag = "8")]
    pub currency: Option<OneItemNpcPurchaseCurrencyV1>,
    #[prost(uint32, tag = "7")]
    pub offer_count: u32,
    #[prost(bool, tag = "1")]
    pub in_backpacks: bool,
    #[prost(bool, tag = "2")]
    pub ignore_cap: bool,
    #[prost(uint32, tag = "3")]
    pub stack_size: u32,
    #[prost(message, optional, tag = "4")]
    pub item_definition: Option<OneItemTypedDefinitionRevisionV1>,
    #[prost(message, optional, tag = "5")]
    pub bag_definition: Option<OneItemTypedDefinitionRevisionV1>,
    #[prost(message, repeated, tag = "6")]
    pub outputs: Vec<OneItemNpcBulkPurchaseOutputV1>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemNpcPurchaseCurrencyV1 {
    #[prost(message, optional, tag = "1")]
    pub definition: Option<OneItemTypedDefinitionRevisionV1>,
    #[prost(message, repeated, tag = "2")]
    pub inputs: Vec<OneItemNpcPurchaseCurrencyInputV1>,
}
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemNpcPurchaseCurrencyInputV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub item_instance_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub parent: Vec<u8>,
    #[prost(bool, tag = "3")]
    pub nested: bool,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemNpcBulkPurchaseOutputV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub item_instance_id: Vec<u8>,
    /// Container and ground are exclusive. Nested bag children use their bag as parent.
    #[prost(bytes = "vec", optional, tag = "2")]
    pub parent: Option<Vec<u8>>,
    #[prost(message, optional, tag = "3")]
    pub ground: Option<OneItemGroundV1>,
}

pub(super) fn check(
    value: &OneItemNpcTradeV1,
    purchase: &OneItemNpcBulkPurchaseV1,
) -> Result<(), AuditError> {
    let bad = || AuditError::InvalidInput;
    if value.lines.len() > 150 {
        return Err(AuditError::CapacityExceeded);
    }
    if value
        .cause
        .as_ref()
        .is_none_or(|cause| cause.side != SIDE_BUY)
        || value.sell_all
        || value.hireling_package.is_some()
        || value.house_key.is_some()
        || value.house_furniture_kit.is_some()
        || !(1..=100).contains(&purchase.stack_size)
        || !(1..=10_000).contains(&value.quantity)
        || purchase.outputs.is_empty()
        || purchase.outputs.len() > 128
    {
        return Err(bad());
    }
    check_definition(purchase.item_definition.as_ref())?;
    let item = purchase.item_definition.as_ref().ok_or_else(bad)?;
    if item.family != "Item" || Coin::from_production_key(&item.production_key).is_some() {
        return Err(bad());
    }
    let count = purchase.offer_count.max(1);
    if count > 100 || !value.quantity.is_multiple_of(count) { return Err(bad()); }
    let stacks = value.quantity.div_ceil(purchase.stack_size);
    let bags = if purchase.in_backpacks {
        stacks.div_ceil(SHOPPING_BAG_SLOTS)
    } else {
        0
    };
    if (if purchase.currency.is_some() { Some(0) } else { value
        .unit_price
        .checked_mul(u64::from(value.quantity / count)) })
        .and_then(|cost| cost.checked_add(u64::from(bags) * SHOPPING_BAG_PRICE))
        != Some(value.total_gold_units)
        || purchase.outputs.len() != usize::try_from(stacks + bags).map_err(|_| bad())?
    {
        return Err(bad());
    }
    if bags > 0 {
        check_definition(purchase.bag_definition.as_ref())?;
        if purchase.bag_definition.as_ref().is_none_or(|bag| {
            bag.family != "Item" || bag.production_key != "oteryn:item.tibia.i21411"
        }) {
            return Err(bad());
        }
    } else if purchase.bag_definition.is_some() {
        return Err(bad());
    }
    let burns: Vec<_> = value.lines.iter().filter(|line| line.kind == LINE_ITEM_BURN).collect();
    if let Some(currency) = &purchase.currency {
        check_definition(currency.definition.as_ref())?;
        let definition = currency.definition.as_ref().ok_or_else(bad)?;
        if definition.family != "Item" || Coin::from_production_key(&definition.production_key).is_some() || burns.len() != currency.inputs.len() { return Err(bad()); }
        let mut spent = 0_u64;
        for (line,input) in burns.iter().zip(&currency.inputs) {
            check_uuid_v7(&input.parent)?;
            let before = line.before.as_ref().ok_or_else(bad)?;
            if before.item_instance_id != input.item_instance_id || before.definition.as_ref() != Some(definition)
                || value.lines.iter().filter(|other| line_item(other) == line_item(line)).count() != 1 { return Err(bad()); }
            spent = spent.checked_add(u64::from(check_burn(line,&value.world_id)?)).ok_or_else(bad)?;
        }
        if Some(spent) != value.unit_price.checked_mul(u64::from(value.quantity/count)) { return Err(bad()); }
    } else if !burns.is_empty() { return Err(bad()); }
    let first_item = value
        .lines
        .iter()
        .position(|line| line.kind == LINE_ITEM_MINT)
        .ok_or_else(bad)?;
    if value.lines[first_item..]
        .iter()
        .any(|line| line.kind != LINE_ITEM_MINT)
    {
        return Err(bad());
    }
    let item_lines: Vec<_> = value
        .lines
        .iter()
        .filter(|line| line.kind == LINE_ITEM_MINT)
        .collect();
    if item_lines.len() != purchase.outputs.len() {
        return Err(bad());
    }
    let mut remaining = value.quantity;
    let mut bag_ids = Vec::new();
    let mut stack_index = 0_u32;
    for (index, (line, output)) in item_lines.iter().zip(&purchase.outputs).enumerate() {
        check_mint(line, &value.world_id)?;
        let after = line.after.as_ref().ok_or_else(bad)?;
        if purchase.outputs[..index]
            .iter()
            .zip(&item_lines[..index])
            .any(|(other, other_line)| {
                output.parent.is_some()
                    && output.parent == other.parent
                    && line.placement_ordinal == other_line.placement_ordinal
            })
        {
            return Err(bad());
        }
        if after.item_instance_id != output.item_instance_id
            || line.placement_ordinal == 0
            || value
                .lines
                .iter()
                .filter(|other| line_item(other) == line_item(line))
                .count()
                != 1
        {
            return Err(bad());
        }
        let is_bag = purchase.in_backpacks && index % (SHOPPING_BAG_SLOTS as usize + 1) == 0;
        if is_bag {
            if after.definition != purchase.bag_definition || after.quantity != 1 {
                return Err(bad());
            }
            bag_ids.push(output.item_instance_id.as_slice());
        } else {
            let units = remaining.min(purchase.stack_size);
            if units == 0 || after.definition.as_ref() != Some(item) || after.quantity != units {
                return Err(bad());
            }
            remaining -= units;
            if purchase.in_backpacks
                && (output.parent.as_deref() != bag_ids.last().copied()
                    || output.ground.is_some()
                    || line.placement_ordinal != u64::from(stack_index % SHOPPING_BAG_SLOTS + 1))
            {
                return Err(bad());
            }
            stack_index += 1;
        }
        if is_bag || !purchase.in_backpacks {
            match (&output.parent, &output.ground) {
                (Some(parent), None) => {
                    check_uuid_v7(parent)?;
                    if purchase
                        .outputs
                        .iter()
                        .any(|output| &output.item_instance_id == parent)
                    {
                        return Err(bad());
                    }
                }
                (None, Some(ground)) if purchase.ignore_cap => {
                    if ground.world_id != value.world_id
                        || ground.channel_id != value.channel_id
                        || ground.runtime_scope_ownership_generation
                            != value.runtime_scope_ownership_generation
                        || ground.spatial_position.len() != 10
                        || ground.instance_id.is_some()
                        || ground.corpse_ref.is_empty()
                        || ground.native_room_placement_context.is_empty()
                    {
                        return Err(bad());
                    }
                    mint::check_technical_text(&ground.map_revision)?;
                    mint::check_technical_text(&ground.content_revision)?;
                }
                _ => return Err(bad()),
            }
        }
    }
    if remaining != 0 || bag_ids.len() != bags as usize {
        return Err(bad());
    }
    // Reuse every existing coin-order, burn-state, change, bank, identity and scope check.
    // The explicit variant independently proves all item outputs above.
    let mut normalized = value.clone();
    normalized.purchase = None;
    normalized.quantity = 1;
    normalized.unit_price = value.total_gold_units;
    normalized.lines.retain(|line| !matches!(line.kind, LINE_ITEM_MINT | LINE_ITEM_BURN));
    let mut output = (*item_lines.last().ok_or_else(bad)?).clone();
    output.placement_ordinal = normalized
        .lines
        .iter()
        .map(|line| line.placement_ordinal)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(bad)?;
    normalized.lines.push(output);
    check_npc_trade(&normalized)
}
