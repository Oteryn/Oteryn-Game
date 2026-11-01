use super::super::character_authority::{ReconciledCharacterAuthority,assert_recovery_fence};
use super::super::character_inbox::{RewardClaimInboxError,RewardClaimInboxMintRequest,RewardClaimInboxOutcome,RewardClaimInboxPlan};
use super::super::db::{begin_semantic_transaction,commit_semantic_transaction,lock_admission_relations};
use super::super::item_transfer::{CurrentCharacterItemFence,ItemWeightLookup,read_derived_transfer_capacity_centi_oz};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError,DurabilityRoot};
use super::{fence,model,receipts};
use oteryn_protocol_oteryn::hireling::management::{Refusal,Reply,Request};
use sqlx::{Postgres,Transaction};

/// Crystal201189f data/items/items.xml:46144-46148. Forced ownership-loss recalls bypass it.
pub const STORE_INBOX_CAPACITY: i64 = 20;

#[derive(Debug,Clone)]
pub struct HirelingLampReturn {
    pub hireling: [u8;16],
    pub input: super::HirelingReturnInput,
    pub source: RewardClaimInboxPlan,
    pub mint: RewardClaimInboxMintRequest,
}

/// Only a typed refusal can be recorded without an accompanying native mutation.
pub struct HirelingRefusal {
    pub binding: [u8; 32],
    pub reason: Refusal,
}

impl DurabilityRoot {
    /// Read only the immutable answer. No inventory or runtime effect is replayed.
    pub async fn read_hireling_command_receipt(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,current:CurrentCharacterItemFence,
        command:u64,request:Request,
    )->Result<Option<Reply>,DurabilityError>{
        self.read_hireling_bound_receipt(authority,current,command,receipts::binding(&request.encode())).await
    }

    pub async fn read_hireling_bound_receipt(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,current:CurrentCharacterItemFence,
        command:u64,digest:[u8;32],
    )->Result<Option<Reply>,DurabilityError>{
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let result=receipts::replay(&mut tx,&current,command,&digest).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await
    }

    /// The caller waits Crystal's1000ms before entering. All live authority and capacity is
    /// checked again in the native Inbox mint transaction; every refusal rolls the mint back.
    pub async fn return_hireling_to_lamp(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,
        current:CurrentCharacterItemFence,request:HirelingLampReturn,weights:&dyn ItemWeightLookup,
    )->Result<Reply,DurabilityError>{
        validate(&request)?;
        let command=request.source.command.command_id().get();
        let digest=request.input.binding(request.hireling);
        if let Some(reply)=self.read_hireling_bound_receipt(authority,current,command,digest).await? {return Ok(reply)}
        let mut candidate=self.prepare_hireling_inbox_mint(authority,node,current,&request.source,request.mint.clone()).await.map_err(inbox_error)?;
        let origin=*request.source.channel_id.as_bytes();
        let mut state=(weights,Reply::Committed);
        let outcome=self.commit_hireling_inbox_mint(authority,node,current,&mut candidate,&mut state,
            move|context,state|Box::pin(async move{
                let tx=context.transaction();
                let row=model::snapshot_query!(" FROM game_house_hirelings h \
                    JOIN game_durability_reconnect_sessions s ON s.game_session_id=encode($2,'hex')::uuid \
                    JOIN game_house_ownership o ON o.world_id=h.world_id AND o.house_key=h.house_key \
                    WHERE h.hireling_id=encode($1,'hex')::uuid AND h.active \
                    AND h.owner_character_id=s.character_id AND h.world_id=s.world_id \
                    AND h.house_key=s.runtime_scope_house_key \
                    AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id \
                    FOR UPDATE OF h FOR SHARE OF o")
                    .bind(request.hireling.as_slice()).bind(current.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
                let Some(row)=row else {state.1=Reply::Refused(Refusal::NotActive);return Err(RewardClaimInboxError::SourceRefused)};
                let h=model::decode(&row)?;
                let reserved:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_hireling_store_reservations WHERE hireling_id=encode($1,'hex')::uuid AND state=1)")
                    .bind(request.hireling.as_slice()).fetch_one(&mut **tx).await?;
                if reserved {state.1=Reply::Refused(Refusal::Conflict);return Err(RewardClaimInboxError::SourceRefused)}
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid")
                    .bind(current.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                if count>STORE_INBOX_CAPACITY {state.1=Reply::Refused(Refusal::InventoryFull);return Err(RewardClaimInboxError::SourceRefused)}
                let carried=super::super::item_loop_move::carried_weight(tx,current.character_id,h.world,state.0).await
                    .map_err(|_|RewardClaimInboxError::SourceRefused)?;
                let capacity=read_derived_transfer_capacity_centi_oz(tx,current.character_id).await
                    .map_err(|_|RewardClaimInboxError::SourceRefused)?;
                let weight=state.0.weight_of(&request.mint.outputs[0].item.definition).map_err(|_|RewardClaimInboxError::SourceRefused)?;
                if carried.checked_add(u64::from(weight)).is_none_or(|total|total>capacity){state.1=Reply::Refused(Refusal::Capacity);return Err(RewardClaimInboxError::SourceRefused)}
                finish_return(tx,&request.source,&h,2).await?;
                receipts::write(tx,&current,command,Some(h.id),digest,&Reply::Committed,None).await?;
                Ok(())
            })).await;
        match outcome {
            Ok(RewardClaimInboxOutcome::Committed{..}|RewardClaimInboxOutcome::AlreadyCommitted(_))=>Ok(Reply::Committed),
            Err(RewardClaimInboxError::SourceRefused) if matches!(state.1,Reply::Refused(_))=>{
                let Reply::Refused(reason) = state.1 else { return Err(DurabilityError::InvalidStoredState) };
                self.record_hireling_refusal(authority,node,current,origin,command,HirelingRefusal { binding:digest, reason }).await
            }
            Err(error)=>Err(inbox_error(error)),
        }
    }

    pub async fn record_hireling_refusal(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,
        current:CurrentCharacterItemFence,origin:[u8;16],command:u64,request:HirelingRefusal,
    )->Result<Reply,DurabilityError>{
        let digest=request.binding;
        let reply=Reply::Refused(request.reason);
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        let node=node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Some(prior)=receipts::replay(&mut tx,&current,command,&digest).await?{return Ok(prior)}
            if !fence::assert_inventory_fence(&mut tx,&node,&current,origin).await?{return Ok(Reply::Refused(Refusal::Stale))}
            receipts::write(&mut tx,&current,command,None,digest,&reply,None).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(reply)
        })).await
    }
}

pub(super) async fn finish_return(
    tx:&mut Transaction<'_,Postgres>,source:&RewardClaimInboxPlan,h:&model::HirelingSnapshot,kind:i16,
)->Result<(),DurabilityError>{
    let position=h.position.ok_or(DurabilityError::InvalidStoredState)?;
    let house=h.house.as_ref().ok_or(DurabilityError::InvalidStoredState)?;
    let lamp=source.item_instance_ids.first().ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_hireling_lamp_transitions \
        (game_session_id,command_id,hireling_id,character_id,world_id,house_key,kind,lamp_item_instance_id,transaction_id,x,y,floor) \
        VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10,$11,$12)")
        .bind(source.command.game_session_id().as_bytes().as_slice()).bind(source.command.command_id().get().to_string())
        .bind(h.id.as_slice()).bind(h.owner.as_slice()).bind(h.world.as_slice()).bind(house).bind(kind)
        .bind(lamp.as_slice()).bind(source.transaction_id.as_slice())
        .bind(i32::from(position.0)).bind(i32::from(position.1)).bind(i16::from(position.2)).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_house_hirelings SET active=FALSE,house_key=NULL,x=0,y=0,floor=0,lamp_item_instance_id=encode($2,'hex')::uuid WHERE hireling_id=encode($1,'hex')::uuid")
        .bind(h.id.as_slice()).bind(lamp.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

pub(super) fn validate(request:&HirelingLampReturn)->Result<(),DurabilityError>{
    if request.source.source_binding!=request.input.binding(request.hireling)
        || request.source.item_instance_ids.len()!=1 || request.mint.outputs.len()!=1
        || request.mint.outputs[0].quantity!=1 || request.mint.outputs[0].item.definition.family!="Item"
        || request.mint.outputs[0].item.definition.production_key!=model::HIRELING_LAMP_KEY {
        return Err(DurabilityError::InvalidStoredState)
    }Ok(())
}
pub(super) fn inbox_error(error:RewardClaimInboxError)->DurabilityError{
    match error{RewardClaimInboxError::Unavailable(error)=>error,_=>DurabilityError::InvalidStoredState}
}
