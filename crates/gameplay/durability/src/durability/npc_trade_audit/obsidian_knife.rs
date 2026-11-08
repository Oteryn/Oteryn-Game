//! Uzgod's exact take/take/give aggregate; no gold or unrelated recipe is admitted.
use super::*;

pub(super) fn check(value: &OneItemNpcTradeV1) -> Result<(), AuditError> {
    let cause = value.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    let [steel, lance, knife] = value.lines.as_slice() else {
        return Err(AuditError::InvalidInput);
    };
    let input = |line: &OneItemNpcTradeLineV1, key: &str| {
        line.before.as_ref().is_some_and(|b| {
            line.kind == LINE_ITEM_BURN
                && b.definition.as_ref().is_some_and(|d| {
                    d.family == "Item"
                        && d.production_key == key
                        && d.revision_ref == "definition-r1"
                })
        })
    };
    let output = knife.after.as_ref().ok_or(AuditError::InvalidInput)?;
    if cause.npc_key != "oteryn:npc.uzgod"
        || cause.catalogue_revision == 0
        || cause.offer_index != 0
        || value.quantity != 1
        || value.unit_price != 0
        || value.sell_all
        || value.bank_debit.is_some()
        || value.house_key.is_some()
        || value.hireling_package.is_some()
        || value.house_furniture_kit.is_some()
        || !input(steel, "oteryn:item.tibia.i5889")
        || !input(lance, "oteryn:item.tibia.i3313")
        || check_burn(steel, &value.world_id)? != 1
        || check_burn(lance, &value.world_id)? != 1
        || lance.before.as_ref().is_none_or(|b| b.quantity != 1)
        || knife.kind != LINE_ITEM_MINT
        || output.quantity != 1
        || !output.definition.as_ref().is_some_and(|d| {
            d.family == "Item"
                && d.production_key == "oteryn:item.tibia.i5908"
                && d.revision_ref == "definition-r1"
        })
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}
