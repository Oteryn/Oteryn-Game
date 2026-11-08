//! Closed barter validates each existing typed BURN/MINT line with the NPC Trade validator.
use super::*;
use crate::durability::npc_trade::yeo::{GLOO, NPC, SHRIMP};
pub(super) fn check(value: &OneItemNpcTradeV1) -> Result<(), AuditError> {
    let cause = value.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    if cause.npc_key != NPC
        || cause.side != SIDE_BUY
        || value.quantity != 1
        || value.unit_price != 0
        || value.total_gold_units != 0
        || value.burned_gold_units != 0
        || value.minted_gold_units != 0
        || value.bank_debit.is_some()
        || value.sell_all
        || value.hireling_package.is_some()
        || value.house_key.is_some()
        || value.house_furniture_kit.is_some()
    {
        return Err(AuditError::InvalidInput);
    }
    let burns = value
        .lines
        .iter()
        .take_while(|line| line.kind == LINE_ITEM_BURN)
        .count();
    if !(1..=10).contains(&burns) || value.lines.len() != burns + 1 {
        return Err(AuditError::InvalidInput);
    }
    let mut paid = 0u32;
    let mut previous = None;
    for line in &value.lines[..burns] {
        let units = check_burn(line, &value.world_id)?;
        let before = line.before.as_ref().ok_or(AuditError::InvalidInput)?;
        let definition = before.definition.as_ref().ok_or(AuditError::InvalidInput)?;
        if definition.family != "Item"
            || definition.production_key != SHRIMP
            || definition.revision_ref != "definition-r1"
            || previous.is_some_and(|ordinal| line.placement_ordinal >= ordinal)
        {
            return Err(AuditError::InvalidInput);
        }
        previous = Some(line.placement_ordinal);
        paid = paid.checked_add(units).ok_or(AuditError::InvalidInput)?;
        // A one-line zero-value ordinary SELL validates this exact typed input state,
        // root/session identities and BURN shape. These are validation views only.
        let mut input = value.clone();
        input.cause.as_mut().ok_or(AuditError::InvalidInput)?.side = SIDE_SELL;
        input.quantity = units;
        input.lines = vec![line.clone()];
        check_ordinary_npc_trade(&input)?;
    }
    if paid != 10 {
        return Err(AuditError::InvalidInput);
    }
    let output = value.lines.last().ok_or(AuditError::InvalidInput)?;
    let after = output.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let definition = after.definition.as_ref().ok_or(AuditError::InvalidInput)?;
    if output.kind != LINE_ITEM_MINT
        || after.quantity != 1
        || definition.family != "Item"
        || definition.production_key != GLOO
        || definition.revision_ref != "definition-r1"
    {
        return Err(AuditError::InvalidInput);
    }
    let mut purchase = value.clone();
    purchase.lines = vec![output.clone()];
    check_ordinary_npc_trade(&purchase)?;
    for (index, line) in value.lines.iter().enumerate() {
        if value.lines[..index]
            .iter()
            .any(|other| line_item(other) == line_item(line))
        {
            return Err(AuditError::InvalidInput);
        }
    }
    check_npc_trade_usage(
        SIDE_BUY,
        false,
        false,
        false,
        false,
        &npc_trade_usage(value),
    )
}
