//! Hirelings extend the existing NPC trade owner with named-House authority and present contents.
use super::*;
use crate::domain::hireling::catalogue::{PACKAGES, PRESENT_ITEM_ID, ROWS};
use crate::foundation::ChannelId;
use super::super::item_transfer::{ItemWeightLookup, validate_facts};
use super::super::npc_trade_audit::OneItemHirelingPackageV1;

#[derive(Debug, Clone)]
pub struct HirelingTradeSelection {
    pub hireling_id:[u8;16],
    pub source_row:usize,
    pub origin_channel:ChannelId,
    /// The pinned Item semantics' full charges. A missing durable timed row means these values.
    pub full_charges:Option<u32>,
    /// Original source target when the native output is decoration kit23398.
    pub wrapped_target:Option<TypedDefinitionRef>,
}

#[derive(Debug, Clone)]
pub struct HirelingPackageSelection {
    pub hireling_id:[u8;16],
    pub package:usize,
    pub origin_channel:ChannelId,
    pub children:[ItemDefinitionFacts;3],
    pub intent_binding:[u8;32],
    pub child_ids:[[u8;16];3],
}

#[derive(Clone)]
pub(super) enum Extra { Trade(HirelingTradeSelection), Package(Box<HirelingPackageSelection>) }

impl Extra {
    pub(super) fn channel(&self)->ChannelId {match self {Self::Trade(s)=>s.origin_channel,Self::Package(s)=>s.origin_channel}}
    pub(super) fn id(&self)->[u8;16] {match self {Self::Trade(s)=>s.hireling_id,Self::Package(s)=>s.hireling_id}}
    pub(super) fn skill(&self)->Option<u16> {match self {Self::Trade(s) if ROWS[s.source_row].category!=0=>Some(1004),_=>None}}
    pub(super) fn validate(&self,request:&NpcTradeRequest,sell_all:bool)->Result<()> {
        if request.cause.npc_key!=super::super::house_hireling::npc_key(self.id()) {return Err(NpcTradeError::InvalidInput)}
        match self {
            Self::Trade(selection)=>{
                let source=ROWS.get(selection.source_row).ok_or(NpcTradeError::InvalidInput)?;
                // Crystal npc.cpp590-595 treats23721 as the all-loot command, never a pouch sale.
                if source.item_id==23721 && request.cause.side==NpcTradeSide::Sell {
                    return Err(NpcTradeError::RequirementNotMet);
                }
                let price=match request.cause.side {NpcTradeSide::Buy=>source.buy,NpcTradeSide::Sell=>source.sell};
                let wrapped = request.cause.side == NpcTradeSide::Buy
                    && crate::domain::hireling::catalogue::wrapped_buy(selection.source_row).is_some();
                let item_id = if wrapped { crate::domain::hireling::catalogue::DECORATION_KIT_ITEM_ID } else { source.item_id };
                if wrapped {
                    let target=selection.wrapped_target.as_ref().ok_or(NpcTradeError::InvalidInput)?;
                    if target.family != "Item" || target.production_key != format!("oteryn:item.tibia.i{}",source.item_id)
                        || target.revision_ref != "definition-r1" || target.revision_ref != request.item.definition.revision_ref
                        || request.quantity != 1 || request.item.container_capacity.is_some() {
                        return Err(NpcTradeError::InvalidInput);
                    }
                } else if selection.wrapped_target.is_some() { return Err(NpcTradeError::InvalidInput); }
                if price==0 || request.unit_price!=price || request.offer_count!=1
                    || request.item.definition.production_key!=format!("oteryn:item.tibia.i{}",item_id)
                    || (source.sub_type!=0 && (request.cause.side!=NpcTradeSide::Buy || selection.full_charges!=Some(source.sub_type))) {
                    return Err(NpcTradeError::InvalidInput);
                }
            }
            Self::Package(selection)=>{
                let package=PACKAGES.get(selection.package).ok_or(NpcTradeError::InvalidInput)?;
                if sell_all || request.cause.side!=NpcTradeSide::Buy || request.quantity!=1 || request.offer_count!=1
                    || request.unit_price!=package.gold || request.item.definition.production_key!=format!("oteryn:item.tibia.i{PRESENT_ITEM_ID}")
                    || request.item.container_capacity.is_none_or(|capacity|capacity<3) {return Err(NpcTradeError::InvalidInput)}
                let mut ids=vec![request.transaction_id,request.event_id];ids.extend(request.output_item_instance_ids);
                for (index,(facts,(id,quantity))) in selection.children.iter().zip(package.items).enumerate() {
                    validate_facts(facts).map_err(|_|NpcTradeError::InvalidInput)?;
                    let (maximum,_)=stack_maximum(facts.stack).map_err(|_|NpcTradeError::InvalidInput)?;
                    if facts.definition.family!="Item" || facts.definition.production_key!=format!("oteryn:item.tibia.i{id}")
                        || quantity==0 || quantity>maximum || facts.container_capacity.is_some()
                        || facts.container_slot_equip_pattern || ids.contains(&selection.child_ids[index])
                        || check_uuid_v7(&selection.child_ids[index]).is_err() {return Err(NpcTradeError::InvalidInput)}
                    ids.push(selection.child_ids[index]);
                }
            }
        }
        Ok(())
    }

    /// Source facts are semantic; output UUIDs retain native trade's retry-local identity policy.
    pub(super) fn bind(&self,previous:Vec<u8>)->Vec<u8> {
        let mut hash=Sha256::new();hash.update(previous);hash.update(b"oteryn:hireling-trade:v1\0");hash.update(self.id());
        match self {
            Self::Trade(s)=>{hash.update([1]);hash.update((s.source_row as u64).to_be_bytes());hash.update(s.full_charges.unwrap_or(0).to_be_bytes());
                if let Some(target)=&s.wrapped_target { hash.update(b"house-furniture-kit:v1");
                    for value in [&target.family,&target.production_key,&target.revision_ref] {
                        hash.update((value.len() as u64).to_be_bytes());hash.update(value.as_bytes());
                    }
                }
            }
            Self::Package(s)=>{hash.update([2]);hash.update((s.package as u64).to_be_bytes());hash.update(s.intent_binding);
                for child in &s.children {for value in [&child.definition.family,&child.definition.production_key,&child.definition.revision_ref] {
                    hash.update((value.len() as u64).to_be_bytes());hash.update(value.as_bytes());
                }}}
        }
        hash.finalize().to_vec()
    }

    pub(super) async fn admit(&self,tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,node:&NodeIncarnationProof,
        fence:&CurrentCharacterItemFence)->Result<bool> {
        Ok(super::super::house_hireling::fence::assert_house_inventory_fence(tx,node,fence,*self.channel().as_bytes()).await?
            && super::super::house_hireling::food::service_in_transaction(tx,fence,self.id(),self.skill()).await?)
    }

    pub(super) async fn package_preflight(&self,tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,
        fence:&CurrentCharacterItemFence,request:&NpcTradeRequest,weights:Option<&dyn ItemWeightLookup>)->Result<()> {
        let Self::Package(selection)=self else {return Ok(())};
        let weights=weights.ok_or(NpcTradeError::InvalidInput)?;
        let mut required=u64::from(request.item.weight.ok_or(NpcTradeError::InvalidInput)?);
        for (child,(_,count)) in selection.children.iter().zip(PACKAGES[selection.package].items) {
            required=required.checked_add(u64::from(child.weight.ok_or(NpcTradeError::InvalidInput)?)*u64::from(count)).ok_or(NpcTradeError::CapacityExceeded)?;
        }
        let carried=super::super::item_loop_move::carried_weight(tx,fence.character_id,*fence.runtime_scope.world_id().as_bytes(),weights)
            .await.map_err(|_|NpcTradeError::CapacityExceeded)?;
        let capacity=super::super::item_transfer::read_derived_transfer_capacity_centi_oz(tx,fence.character_id).await?;
        if carried.checked_add(required).is_none_or(|total|total>capacity) {return Err(NpcTradeError::DoesNotFit)}
        let ids:Vec<String>=selection.child_ids.iter().map(|id|id.iter().map(|b|format!("{b:02x}")).collect()).collect();
        let used:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=ANY($1::text[]::uuid[]))")
            .bind(ids).fetch_one(&mut **tx).await?;
        if used {return Err(NpcTradeError::ConflictingCandidate)}
        Ok(())
    }

    pub(super) fn kit_audit(&self,item:[u8;16])->Option<super::super::npc_trade_audit::OneItemHouseFurnitureKitV1> {
        let Self::Trade(selection)=self else {return None};
        let target=selection.wrapped_target.as_ref()?;
        Some(super::super::npc_trade_audit::OneItemHouseFurnitureKitV1 {
            item_instance_id:item.to_vec(),source_row:u32::try_from(selection.source_row).ok()?,
            target:Some(OneItemTypedDefinitionRevisionV1 {family:target.family.clone(),
                production_key:target.production_key.clone(),revision_ref:target.revision_ref.clone()}),
        })
    }

    pub(super) async fn apply_materialized(&self,tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,world:WorldId,
        request:&NpcTradeRequest,item:[u8;16])->Result<()> {
        if let Self::Trade(selection)=self && let Some(target)=&selection.wrapped_target {
            super::super::house_wrap::record_purchased_kit(tx,request.transaction_id,item,selection.source_row,target).await?;
        }
        self.apply_package(tx,world,request,item).await
    }

    pub(super) fn package_audit(&self,world:WorldId,present:[u8;16])->Option<OneItemHirelingPackageV1> {
        let Self::Package(selection)=self else {return None};
        let package=&PACKAGES[selection.package];
        Some(OneItemHirelingPackageV1 {hireling_id:selection.hireling_id.to_vec(),name:package.name.into(),present_item_instance_id:present.to_vec(),
            children:selection.children.iter().zip(package.items).enumerate().map(|(index,(facts,(_,quantity)))|OneItemNpcTradeLineV1 {
                kind:LINE_ITEM_MINT,before:None,placement_ordinal:index as u64+1,after:Some(OneItemStateV1 {
                    item_instance_id:selection.child_ids[index].to_vec(),world_id:world.as_bytes().to_vec(),quantity,lifecycle:ITEM_LIFECYCLE_LIVE,key_number:0,
                    definition:Some(OneItemTypedDefinitionRevisionV1 {family:facts.definition.family.clone(),production_key:facts.definition.production_key.clone(),revision_ref:facts.definition.revision_ref.clone()})})
            }).collect()})
    }

    pub(super) async fn apply_package(&self,tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,world:WorldId,
        request:&NpcTradeRequest,present:[u8;16])->Result<()> {
        let Self::Package(selection)=self else {return Ok(())};
        sqlx::query("INSERT INTO game_hireling_package_receipts(transaction_id,hireling_id,package_index,present_item_instance_id,intent_binding) \
            VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,$5)")
            .bind(request.transaction_id.as_slice()).bind(selection.hireling_id.as_slice()).bind(selection.package as i16).bind(present.as_slice()).bind(selection.intent_binding.as_slice())
            .execute(&mut **tx).await?;
        for (index,(facts,(_,quantity))) in selection.children.iter().zip(PACKAGES[selection.package].items).enumerate() {
            let id=selection.child_ids[index];
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) \
                VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,1,encode($7,'hex')::uuid)")
                .bind(id.as_slice()).bind(world.as_bytes().as_slice()).bind(&facts.definition.family).bind(&facts.definition.production_key)
                .bind(&facts.definition.revision_ref).bind(i64::from(quantity)).bind(request.transaction_id.as_slice()).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) \
                VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid)")
                .bind(id.as_slice()).bind(world.as_bytes().as_slice()).bind(present.as_slice()).bind(index as i64+1).bind(request.transaction_id.as_slice())
                .execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_hireling_package_lines(transaction_id,line_ordinal,item_instance_id,definition_revision) \
                VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4)")
                .bind(request.transaction_id.as_slice()).bind(index as i16+1).bind(id.as_slice()).bind(&facts.definition.revision_ref).execute(&mut **tx).await?;
        }
        Ok(())
    }
}

impl DurabilityRoot {
    pub async fn read_hireling_package_receipt(&self,authority:&ReconciledCharacterAuthority<'_, '_>,
        character:CharacterId,command:CommandRef,intent_binding:[u8;32])->Result<bool> {
        let recovery=authority.record_for(self).map_err(|_|NpcTradeError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
            let found:Option<Vec<u8>>=sqlx::query_scalar("SELECT p.intent_binding FROM game_hireling_package_receipts p JOIN game_npc_trades t USING(transaction_id) WHERE t.character_id=encode($1,'hex')::uuid AND t.game_session_id=encode($2,'hex')::uuid AND t.command_id=$3::text::numeric")
                .bind(character.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            let result=match found {None=>Ok(false),Some(binding) if binding==intent_binding=>Ok(true),Some(_)=>Err(NpcTradeError::ConflictingOccurrence)};
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await?
    }
    pub async fn commit_hireling_trade(&self,authority:&ReconciledCharacterAuthority<'_, '_>,node:&NodeIncarnationProof,
        fence:CurrentCharacterItemFence,request:NpcTradeRequest,selection:HirelingTradeSelection,sell_all:bool)->Result<NpcTradeOutcome> {
        self.commit_trade(authority,node,fence,request,TradeCommitOptions { sell_all, hireling:Some(Extra::Trade(selection)), weights:None }).await
    }
    pub async fn commit_hireling_package(&self,authority:&ReconciledCharacterAuthority<'_, '_>,node:&NodeIncarnationProof,
        fence:CurrentCharacterItemFence,request:NpcTradeRequest,selection:HirelingPackageSelection,
        weights:&dyn ItemWeightLookup)->Result<NpcTradeOutcome> {
        self.commit_trade(authority,node,fence,request,TradeCommitOptions { sell_all:false, hireling:Some(Extra::Package(Box::new(selection))), weights:Some(weights) }).await
    }
}
