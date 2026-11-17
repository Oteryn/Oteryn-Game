//! A separately proven token payment; shopping bags always use the existing gold fee owner.
use super::*;
use super::super::super::npc_trade_audit::purchase::{
    OneItemNpcPurchaseCurrencyV1, OneItemNpcPurchaseCurrencyInputV1,
};

pub fn currency_message(amount: u32, name: &str, cost: u64, currency: &str, bags: u32) -> String {
    let mut text = format!("Bought {amount}x {name} for {cost} {currency}{}", if cost > 1 { "s" } else { "" });
    if bags > 0 {
        text.push_str(&format!(" and {bags}x shopping bag{} for {} gold coin{}", if bags > 1 { "s" } else { "" }, bags * 20, if bags * 20 > 1 { "s" } else { "" }));
    }
    text.push('.');
    text
}

#[allow(clippy::too_many_arguments, reason = "one fenced payment preparation with locked carried rows")]
pub(super) async fn plan(
    tx: &mut Type2Transaction<'_>, fence: &CurrentCharacterItemFence,
    _request: &NpcTradeRequest, currency: Option<&PurchaseCurrency>, cost: u64,
    rows: &[sqlx::postgres::PgRow], plan: &mut Plan, carried: &mut u64, main_free: &mut usize,
) -> Result<Option<OneItemNpcPurchaseCurrencyV1>> {
    let Some(currency) = currency else { return Ok(None); };
    if is_coin(&currency.facts.definition) { return Err(NpcTradeError::InvalidInput); }
    let unit = u64::from(currency.facts.weight.ok_or(NpcTradeError::InvalidInput)?);
    let mut remaining = cost;
    let mut inputs = Vec::new();
    for row in rows {
        if remaining == 0 { break; }
        if row_definition(row)? != currency.facts.definition { continue; }
        let id = uuid_text(&row.try_get::<String,_>("item_instance_id")?)?;
        // Only owned container contents can pay. Equipped roots and nonempty containers cannot.
        let location = sqlx::query("SELECT parent_item_instance_id::text,placement_ordinal::text,false AS nested FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid UNION ALL SELECT parent_item_instance_id::text,placement_ordinal::text,true AS nested FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let Some(location) = location else { continue; };
        let nonempty: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)").bind(id.as_slice()).fetch_one(&mut **tx).await?;
        if nonempty { continue; }
        let before = u32::try_from(row.try_get::<i64,_>("quantity")?).map_err(|_| NpcTradeError::InvalidInput)?;
        let spent = u32::try_from(remaining.min(u64::from(before))).map_err(|_| NpcTradeError::InvalidInput)?;
        let nested = location.try_get::<bool,_>("nested")?;
        plan.lines.push(NpcTradeLine { kind: NpcTradeLineKind::ItemBurn, item_instance_id: id,
            placement_ordinal: numeric_u64(&location,"placement_ordinal")?, coin: None,
            quantity_before: before, quantity_after: before - spent });
        inputs.push(OneItemNpcPurchaseCurrencyInputV1 { item_instance_id: id.to_vec(), parent: uuid_text(&location.try_get::<String,_>("parent_item_instance_id")?)?.to_vec(), nested });
        if !nested && before == spent { *main_free += 1; }
        *carried = carried.checked_sub(unit * u64::from(spent)).ok_or(NpcTradeError::InvalidInput)?;
        remaining -= u64::from(spent);
    }
    if remaining != 0 { return Err(NpcTradeError::InsufficientFunds); }
    // The native request binding already covers the offer's identity and unit price.
    Ok(Some(OneItemNpcPurchaseCurrencyV1 { definition: Some(definition_message(&currency.facts.definition)), inputs }))
}

pub(super) async fn store(tx: &mut sqlx::Transaction<'_,sqlx::Postgres>, request: &NpcTradeRequest, prepared: &PreparedPurchase) -> std::result::Result<(),DurabilityError> {
    if let Some(currency) = &prepared.audit.currency {
        for input in &currency.inputs {
            sqlx::query("INSERT INTO game_npc_purchase_currency_inputs(transaction_id,item_instance_id,parent_id,nested) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
                .bind(request.transaction_id.as_slice()).bind(&input.item_instance_id).bind(&input.parent).bind(input.nested).execute(&mut **tx).await?;
        }
    }
    Ok(())
}
