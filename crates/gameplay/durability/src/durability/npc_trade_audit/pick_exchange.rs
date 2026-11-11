//! Closed audit shape for Al Dee; unknown recipes cannot enter this source.
use super::*;

pub(super) fn check(value: &OneItemNpcTradeV1) -> Result<(), AuditError> {
    check_recipe(
        value,
        "oteryn:npc.al_dee",
        "oteryn:item.tibia.i3462",
        "oteryn:item.tibia.i3456",
    )
}

pub(super) fn check_recipe(
    value: &OneItemNpcTradeV1,
    npc: &str,
    input: &str,
    output: &str,
) -> Result<(), AuditError> {
    let cause = value.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    let [take, give] = value.lines.as_slice() else {
        return Err(AuditError::InvalidInput);
    };
    let before = take.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let after = give.after.as_ref().ok_or(AuditError::InvalidInput)?;
    let matches = |state: &OneItemStateV1, key: &str| {
        state.definition.as_ref().is_some_and(|d| {
            d.family == "Item" && d.production_key == key && d.revision_ref == "definition-r1"
        })
    };
    if cause.npc_key != npc
        || cause.catalogue_revision == 0
        || cause.offer_index != 0
        || value.quantity != 1
        || value.unit_price != 0
        || value.sell_all
        || value.bank_debit.is_some()
        || value.house_key.is_some()
        || value.hireling_package.is_some()
        || value.house_furniture_kit.is_some()
        || take.kind != LINE_ITEM_BURN
        || give.kind != LINE_ITEM_MINT
        || before.quantity != 1
        || after.quantity != 1
        || !matches(before, input)
        || !matches(after, output)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}
