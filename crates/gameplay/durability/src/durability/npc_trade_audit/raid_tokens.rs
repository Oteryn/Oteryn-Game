//! Narrow C8 audit shape: one source token BURN and one replica MINT, no value lines.
use super::*;
use oteryn_gameplay_quests::quest::raid_tokens as source;

pub(super) fn check(value: &OneItemNpcTradeV1) -> Result<(), AuditError> {
    let invalid = AuditError::InvalidInput;
    let cause = value.cause.as_ref().ok_or(invalid)?;
    let offers = source::offers().map_err(|_| invalid)?;
    let (output, _) = offers.get(cause.offer_index as usize).ok_or(invalid)?;
    let command = value.command.as_ref().ok_or(invalid)?;
    if cause.npc_key != source::NPC
        || cause.catalogue_revision != 1
        || command.command_id == 0
        || value.runtime_scope_ownership_generation == 0
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
        || value.lines.len() != 2
    {
        return Err(invalid);
    }
    let [burn, mint] = value.lines.as_slice() else {
        return Err(invalid);
    };
    if burn.kind != LINE_ITEM_BURN
        || mint.kind != LINE_ITEM_MINT
        || check_burn(burn, &value.world_id)? != source::COST
        || check_mint(mint, &value.world_id)? != 1
        || mint.placement_ordinal <= burn.placement_ordinal
        || line_item(burn) == line_item(mint)
    {
        return Err(invalid);
    }
    for (line, expected) in [(burn, source::TOKEN), (mint, *output)] {
        let state = line.after.as_ref().ok_or(invalid)?;
        let definition = state.definition.as_ref().ok_or(invalid)?;
        if definition.family != "Item"
            || definition.production_key != format!("oteryn:item.tibia.i{expected}")
        {
            return Err(invalid);
        }
    }
    let usage = npc_trade_usage(value);
    if usage.touched_item_instances > 6
        || usage.location_custody_lines > 6
        || usage.participants > 6
        || usage.effect_work_units > 15
        || usage.events > 1
        || usage.value_lines != 0
        || usage.transform_lines != 0
        || usage.container_expansion != 0
    {
        return Err(AuditError::CapacityExceeded);
    }
    Ok(())
}
