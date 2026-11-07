use super::super::character_authority::{ReconciledCharacterAuthority,assert_recovery_fence};
use super::super::db::{begin_semantic_transaction,commit_semantic_transaction,lock_admission_relations};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError,DurabilityRoot};
use super::{fence,model,receipts};
use oteryn_protocol_oteryn::hireling::management::{Refusal,Reply,Request};

impl DurabilityRoot {
    /// Character inventory or owned named House. The exact list answer is recorded for replay.
    pub async fn query_hirelings(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,
        current:CurrentCharacterItemFence,origin_channel:[u8;16],command:u64,
    )->Result<Reply,DurabilityError>{
        self.query_hirelings_scope(authority,node,current,origin_channel,command,false).await
    }

    /// Command220 answers its original House snapshot even if a later command returns a lamp.
    pub async fn query_legacy_hireling(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,
        current:CurrentCharacterItemFence,origin_channel:[u8;16],command:u64,
    )->Result<Reply,DurabilityError>{
        self.query_hirelings_scope(authority,node,current,origin_channel,command,true).await
    }

    async fn query_hirelings_scope(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,
        current:CurrentCharacterItemFence,origin_channel:[u8;16],command:u64,house_only:bool,
    )->Result<Reply,DurabilityError>{
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        let node=node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            let digest=if house_only {receipts::binding(b"legacy-query-client-input-v1\0")} else {receipts::binding(&Request::List.encode())};
            if let Some(reply)=receipts::replay(&mut tx,&current,command,&digest).await? {
                commit_semantic_transaction(tx,deadline).await?;return Ok(reply)
            }
            if !fence::assert_inventory_fence(&mut tx,&node,&current,origin_channel).await? {
                return Ok(Reply::Refused(Refusal::Stale))
            }
            let rows=if house_only {
                model::snapshot_query!(" FROM game_house_hirelings h \
                    JOIN game_durability_reconnect_sessions s ON s.game_session_id=encode($1,'hex')::uuid \
                    WHERE h.active AND h.world_id=s.world_id AND h.house_key=s.runtime_scope_house_key \
                    AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id \
                    ORDER BY h.hireling_id LIMIT 11")
                    .bind(current.game_session_id.as_bytes().as_slice()).fetch_all(&mut *tx).await?
            } else {
                model::snapshot_query!(" FROM game_house_hirelings h \
                    WHERE owner_character_id=encode($1,'hex')::uuid ORDER BY hireling_id LIMIT 11")
                    .bind(current.character_id.as_bytes().as_slice()).fetch_all(&mut *tx).await?
            };
            if rows.len()>model::MAX_HIRELINGS{return Err(DurabilityError::InvalidStoredState)}
            let entries=rows.iter().map(|r|model::decode(r).map(receipts::entry)).collect::<Result<Vec<_>,_>>()?;
            let reply=Reply::State(entries);
            receipts::write(&mut tx,&current,command,None,digest,&reply,None).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(reply)
        })).await
    }

    pub async fn set_hireling_outfit(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,
        current:CurrentCharacterItemFence,origin_channel:[u8;16],command:u64,request:Request,
    )->Result<Reply,DurabilityError>{
        let Request::Outfit{hireling,look_type,colors}=request else{return Err(DurabilityError::InvalidStoredState)};
        if Request::decode(&request.encode())!=Some(request){return Err(DurabilityError::InvalidStoredState)}
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        let node=node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            let digest=receipts::binding(&request.encode());
            if let Some(reply)=receipts::replay(&mut tx,&current,command,&digest).await? {
                commit_semantic_transaction(tx,deadline).await?;return Ok(reply)
            }
            if !fence::assert_house_inventory_fence(&mut tx,&node,&current,origin_channel).await? {
                return Ok(Reply::Refused(Refusal::Stale))
            }
            let row=model::snapshot_query!(" FROM game_house_hirelings h \
                JOIN game_durability_reconnect_sessions s ON s.game_session_id=encode($2,'hex')::uuid \
                JOIN game_house_ownership o ON o.world_id=h.world_id AND o.house_key=h.house_key \
                WHERE h.hireling_id=encode($1,'hex')::uuid AND h.world_id=s.world_id \
                AND h.house_key=s.runtime_scope_house_key AND h.owner_character_id=s.character_id \
                AND game_house_effective_owner(h.world_id,h.house_key)=h.owner_character_id \
                FOR UPDATE OF h FOR SHARE OF o")
                .bind(hireling.as_slice()).bind(current.game_session_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let snapshot=row.as_ref().map(model::decode).transpose()?;
            let reserved:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_hireling_store_reservations WHERE hireling_id=encode($1,'hex')::uuid AND state=1)")
                .bind(hireling.as_slice()).fetch_one(&mut *tx).await?;
            let reply=match snapshot.as_ref() {
                Some(_) if reserved=>Reply::Refused(Refusal::Conflict),
                None=>Reply::Refused(Refusal::NotOwner),
                Some(h) if h.position.is_none()=>Reply::Refused(Refusal::NotActive),
                Some(h) if !h.allows_outfit(look_type)=>Reply::Refused(Refusal::OutfitNotOwned),
                Some(_)=>{
                    sqlx::query("UPDATE game_house_hirelings SET look_type=$2,colors=$3 WHERE hireling_id=encode($1,'hex')::uuid")
                        .bind(hireling.as_slice()).bind(i32::from(look_type)).bind(colors.as_slice()).execute(&mut *tx).await?;
                    Reply::Committed
                }
            };
            receipts::write(&mut tx,&current,command,snapshot.as_ref().map(|h|h.id),digest,&reply,snapshot.and_then(|h|h.position)).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(reply)
        })).await
    }
}
