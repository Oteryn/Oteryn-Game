//! Crystal's bounded physical payout, composed with the native SELL burn planner.
//!
//! ANSWER Q8 on PR2783: item43963 is the largest offer (5,400,000 gold). No bank or
//! ground fallback is admitted. Existing SELL_ALL audit/resource ceilings still apply.
use super::*;
use crate::domain::currency::COIN_STACK_MAXIMUM;

pub(crate) const MAX_GOLD: u64 = 5_400_000;
pub(crate) const MAX_COIN_STACKS: usize = 8;

/// Highest denomination first, full crystal stacks before its final partial stack.
/// Even MAX_GOLD-1 has at most six crystal stacks, one platinum and one gold.
pub(crate) fn coins(value: u64) -> Option<Vec<(Coin, u32)>> {
    if value > MAX_GOLD { return None; }
    let mut crystal=value / Coin::Crystal.worth();
    let mut result=Vec::with_capacity(MAX_COIN_STACKS);
    while crystal>0 {
        let quantity=crystal.min(u64::from(COIN_STACK_MAXIMUM));
        result.push((Coin::Crystal,u32::try_from(quantity).ok()?));
        crystal-=quantity;
    }
    let platinum=(value % Coin::Crystal.worth()) / Coin::Platinum.worth();
    let gold=value % Coin::Platinum.worth();
    for (coin,quantity) in [(Coin::Platinum,platinum),(Coin::Gold,gold)] {
        if quantity>0 {result.push((coin,u32::try_from(quantity).ok()?));}
    }
    (result.len()<=MAX_COIN_STACKS).then_some(result)
}

pub(super) fn selected(extra:Option<&hireling::Extra>,request:&NpcTradeRequest)->bool {
    request.cause.side==NpcTradeSide::Sell && matches!(extra,Some(hireling::Extra::Trade(_)))
}

/// Native three output identities are retained; only the five additional crystal slots derive
/// new identities from the same immutable CommandRef. No client-provided identity is trusted.
pub(super) fn output_ids(request:&NpcTradeRequest)->[[u8;16];MAX_COIN_STACKS] {
    let mut ids=[[0;16];MAX_COIN_STACKS];
    ids[0]=request.output_item_instance_ids[0];
    ids[6]=request.output_item_instance_ids[1];
    ids[7]=request.output_item_instance_ids[2];
    for (ordinal,id) in ids.iter_mut().enumerate().take(6).skip(1) {
        let hash=Sha256::new().chain_update(b"oteryn:hireling-npc-sell-crystal-stack:v1\0")
            .chain_update(request.command.game_session_id().as_bytes())
            .chain_update(request.command.command_id().get().to_be_bytes())
            .chain_update((ordinal as u64).to_be_bytes()).finalize();
        id.copy_from_slice(&hash[..16]);id[6]=(id[6]&0x0f)|0x70;id[8]=(id[8]&0x3f)|0x80;
    }
    ids
}

pub(super) fn validate(request:&NpcTradeRequest,sell_all:bool)->Result<Option<(u64,u32)>> {
    let value=super::validate_sell_bound(request,sell_all,MAX_GOLD)?;
    let ids=output_ids(request);
    if ids.iter().enumerate().any(|(index,id)|ids[..index].contains(id)
        || id==&request.transaction_id || id==&request.event_id || check_uuid_v7(id).is_err()) {
        return Err(NpcTradeError::InvalidInput);
    }
    Ok(value)
}

pub(super) async fn identities_used(tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,request:&NpcTradeRequest)->std::result::Result<bool,DurabilityError> {
    let ids:Vec<String>=output_ids(request).iter().map(|id|id.iter().map(|b|format!("{b:02x}")).collect()).collect();
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=ANY($1::text[]::uuid[]))")
        .bind(ids).fetch_one(&mut **tx).await?)
}

pub(super) fn plan(request:&NpcTradeRequest,priced:Option<(u64,u32)>,capacity:usize,entries:&[Entry])->Result<(Plan,u64,u32)> {
    // The existing planner selects and validates exactly the same BURN lines. Zero proceeds
    // leave only those lines; this extension materializes their bounded denomination payout.
    let (native,total,units)=match priced {
        Some((total,units))=>(rules::plan_sell(&plan_input(request),0,units,capacity,entries)?,total,units),
        None=>{
            let (native,units)=rules::plan_sell_all(&plan_input(request),0,capacity,entries)?;
            let total=request.unit_price.checked_mul(u64::from(units)).ok_or(NpcTradeError::InvalidInput)?;
            (native,total,units)
        }
    };
    let payout=coins(total).ok_or(NpcTradeError::CapacityExceeded)?;
    let whole=native.lines.iter().filter(|line|line.quantity_after==0).count();
    if entries.len().checked_sub(whole).and_then(|used|used.checked_add(payout.len())).is_none_or(|used|used>capacity) {
        return Err(NpcTradeError::DoesNotFit);
    }
    let mut ordinal=entries.first().map_or(0,|entry|entry.placement_ordinal)
        .checked_add(1).ok_or(NpcTradeError::DoesNotFit)?;
    let mut plan:Plan=native.into();
    let ids=output_ids(request);let mut crystals=0;
    for (coin,quantity) in payout {
        let index=match coin {Coin::Crystal=>{let index=crystals;crystals+=1;index},Coin::Platinum=>6,Coin::Gold=>7};
        plan.lines.push(NpcTradeLine {kind:NpcTradeLineKind::CoinMint,item_instance_id:ids[index],placement_ordinal:ordinal,
            coin:Some(coin),quantity_before:0,quantity_after:quantity});
        ordinal=ordinal.checked_add(1).ok_or(NpcTradeError::DoesNotFit)?;
    }
    plan.minted_gold_units=total;
    Ok((plan,total,units))
}

/// Audit has no runtime authority. It recognizes only the canonical Hireling identity and an
/// executable, matching donor sell price/item; the durable writer separately proves live House.
pub(crate) fn audit_source(trade:&OneItemNpcTradeV1)->bool {
    let Some(cause)=trade.cause.as_ref() else{return false};
    if cause.side!=SIDE_SELL || trade.hireling_package.is_some()
        || super::super::house_hireling::npc_identity(&cause.npc_key).is_none_or(|id|check_uuid_v7(&id).is_err()) {return false}
    let Some(definition)=trade.lines.iter().find(|line|line.kind==LINE_ITEM_BURN)
        .and_then(|line|line.before.as_ref()).and_then(|state|state.definition.as_ref()) else{return false};
    let Some(item_id)=definition.production_key.strip_prefix("oteryn:item.tibia.i").and_then(|id|id.parse::<u32>().ok()) else{return false};
    // Crystal23721 is a virtual all-loot command, never a physical pouch BURN.
    item_id!=23721 && definition.family=="Item" && crate::domain::hireling::catalogue::ROWS.iter()
        .any(|row|row.item_id==item_id && row.sell>0 && row.sell==trade.unit_price)
}

#[cfg(test)]
#[path="sell_hireling/tests.rs"]
mod tests;
