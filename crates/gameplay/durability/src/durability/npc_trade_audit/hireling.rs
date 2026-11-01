//! Closed extension of native NPC trade: one present containing the donor's three stacks.
use super::*;
use super::super::item_mint_audit::OneItemTypedDefinitionRevisionV1;
use crate::domain::hireling::catalogue::{PACKAGES, PRESENT_ITEM_ID};

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemHirelingPackageV1 {
    #[prost(bytes="vec", tag="1")]
    pub hireling_id: Vec<u8>,
    #[prost(string, tag="2")]
    pub name: String,
    #[prost(bytes="vec", tag="3")]
    pub present_item_instance_id: Vec<u8>,
    #[prost(message, repeated, tag="4")]
    pub children: Vec<OneItemNpcTradeLineV1>,
}

/// Original target of one native decoration-kit MINT, owned by house_wrap.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemHouseFurnitureKitV1 {
    #[prost(bytes="vec", tag="1")]
    pub item_instance_id: Vec<u8>,
    #[prost(message, optional, tag="2")]
    pub target: Option<OneItemTypedDefinitionRevisionV1>,
    #[prost(uint32, tag="3")]
    pub source_row: u32,
}

pub(super) fn check_kit(trade:&OneItemNpcTradeV1)->Result<(),AuditError> {
    let Some(kit)=&trade.house_furniture_kit else {return Ok(())};
    check_uuid_v7(&kit.item_instance_id)?;
    let target=kit.target.as_ref().ok_or(AuditError::InvalidInput)?;
    check_definition(Some(target))?;
    let source=crate::domain::hireling::catalogue::wrapped_buy(kit.source_row as usize)
        .ok_or(AuditError::InvalidInput)?;
    let cause=trade.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    let minted=trade.lines.iter().find(|line|line.kind==LINE_ITEM_MINT)
        .and_then(|line|line.after.as_ref()).ok_or(AuditError::InvalidInput)?;
    if trade.hireling_package.is_some() || trade.sell_all || cause.side!=SIDE_BUY
        || crate::durability::house_hireling::npc_identity(&cause.npc_key).is_none()
        || trade.quantity!=1 || trade.unit_price!=source.buy || minted.quantity!=1
        || minted.item_instance_id!=kit.item_instance_id || target.family!="Item"
        || target.production_key!=format!("oteryn:item.tibia.i{}",source.item_id)
        || target.revision_ref!="definition-r1"
        || minted.definition.as_ref().is_none_or(|definition|definition.family!="Item"
            || definition.production_key!="oteryn:item.tibia.i23398"
            || definition.revision_ref!=target.revision_ref) {return Err(AuditError::InvalidInput)}
    Ok(())
}

pub(super) fn check_package(trade: &OneItemNpcTradeV1)->Result<(),AuditError> {
    let Some(package)=trade.hireling_package.as_ref() else {return Ok(())};
    check_uuid_v7(&package.hireling_id)?;
    check_uuid_v7(&package.present_item_instance_id)?;
    let source=PACKAGES.iter().find(|source|source.name==package.name).ok_or(AuditError::InvalidInput)?;
    let cause=trade.cause.as_ref().ok_or(AuditError::InvalidInput)?;
    let mut id=[0;16]; id.copy_from_slice(&package.hireling_id);
    if cause.npc_key != crate::durability::house_hireling::npc_key(id)
        || cause.side!=SIDE_BUY || trade.sell_all || trade.quantity!=1 || trade.unit_price!=source.gold
        || package.children.len()!=3 {return Err(AuditError::InvalidInput)}
    let present=trade.lines.iter().find(|line|line.kind==LINE_ITEM_MINT).and_then(|line|line.after.as_ref())
        .ok_or(AuditError::InvalidInput)?;
    if present.item_instance_id!=package.present_item_instance_id || present.quantity!=1
        || present.definition.as_ref().is_none_or(|definition|definition.production_key!=format!("oteryn:item.tibia.i{PRESENT_ITEM_ID}")) {
        return Err(AuditError::InvalidInput);
    }
    let mut ids=trade.lines.iter().filter_map(line_item).collect::<Vec<_>>();
    for (index,(line,(item,quantity))) in package.children.iter().zip(source.items).enumerate() {
        if line.kind!=LINE_ITEM_MINT || line.placement_ordinal!=index as u64+1
            || check_mint(line,&trade.world_id)?!=quantity {return Err(AuditError::InvalidInput)}
        let after=line.after.as_ref().ok_or(AuditError::InvalidInput)?;
        if after.definition.as_ref().is_none_or(|definition|definition.family!="Item" || definition.production_key!=format!("oteryn:item.tibia.i{item}"))
            || ids.contains(&after.item_instance_id.as_slice()) {return Err(AuditError::InvalidInput)}
        ids.push(after.item_instance_id.as_slice());
    }
    Ok(())
}
