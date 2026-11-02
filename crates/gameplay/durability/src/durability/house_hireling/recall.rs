//! Ownership-loss recovery is fenced by the current House owner node, not an offline session.
use super::super::character_authority::{ReconciledCharacterAuthority,assert_recovery_fence};
use super::super::character_inbox::{HirelingRecallRequest,RewardClaimInboxError,RewardClaimInboxOutcome,RewardClaimInboxPlan};
use super::super::character_progression::numeric_u64;
use super::super::db::{begin_semantic_transaction,commit_semantic_transaction,lock_admission_relations};
use super::super::runtime_scope_assignment::{NodeIncarnationProof,prove_current_incarnation};
use super::super::{DurabilityError,DurabilityRoot};
use super::{HirelingSnapshot,HirelingLampReturn,model,receipts,return_lamp};
use crate::foundation::{ChannelId,CommandId,CommandRef,GameSessionId,WorldId};
use sqlx::Row;

#[derive(Debug,Clone)]
pub struct HirelingRecallState {
    pub hireling:HirelingSnapshot,
    pub authority:HirelingRecallRequest,
    pub source:RewardClaimInboxPlan,
    pub content_revision:String,
    pub ruleset_revision:String,
}

impl DurabilityRoot {
    /// At most ten recalls per pass. A caller drains successive batches; no source row is
    /// truncated or acknowledged until its native Inbox return actually commits.
    pub async fn pending_hireling_recalls(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,world:WorldId,
    )->Result<Vec<HirelingRecallState>,DurabilityError>{
        let recovery=authority.record_for(self).map_err(|_|DurabilityError::InvalidStoredState)?;
        let node=node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx,&node).await? {return Err(DurabilityError::InvalidStoredState)}
            let fact=node.fact();
            let rows=model::snapshot_query!(",a.ownership_generation::text AS house_generation, \
                uuid_send(h.last_lamp_session) AS source_session,h.last_lamp_command::text AS source_command, \
                uuid_send(h.last_lamp_channel) AS source_channel,r.content_revision,r.ruleset_revision \
                FROM game_house_hirelings h JOIN game_character_roots r ON r.character_id=h.owner_character_id \
                JOIN game_runtime_scope_assignments a ON a.scope_kind=2 AND a.world_id=h.world_id AND a.house_key=h.house_key \
                WHERE h.world_id=encode($1,'hex')::uuid AND h.active AND a.state=1 \
                AND h.last_lamp_session IS NOT NULL \
                AND a.holder_node_id=encode($2,'hex')::uuid AND a.holder_registration_revision=$3::text::numeric \
                AND game_house_effective_owner(h.world_id,h.house_key) IS DISTINCT FROM h.owner_character_id \
                ORDER BY h.hireling_id LIMIT 10")
                .bind(world.as_bytes().as_slice()).bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string()).fetch_all(&mut *tx).await?;
            let mut result=Vec::with_capacity(rows.len());
            for row in rows {
                let h=model::decode(&row)?;
                let session:Vec<u8>=row.try_get("source_session")?;
                let channel:Vec<u8>=row.try_get("source_channel")?;
                let command=CommandRef::new(GameSessionId::decode(&session).map_err(|_|DurabilityError::InvalidStoredState)?,
                    CommandId::new(numeric_u64(&row,"source_command")?).map_err(|_|DurabilityError::InvalidStoredState)?);
                let character=crate::domain::CharacterId::from_bytes(h.owner).map_err(|_|DurabilityError::InvalidStoredState)?;
                let recall=HirelingRecallRequest{hireling:h.id,house:h.house.clone().ok_or(DurabilityError::InvalidStoredState)?,scope_ownership_generation:numeric_u64(&row,"house_generation")?};
                let source=RewardClaimInboxPlan{command,character_id:character,world_id:world,
                    channel_id:ChannelId::decode(&channel).map_err(|_|DurabilityError::InvalidStoredState)?,
                    transaction_id:receipts::command_identity(command,b"lamp-forced-return"),source_binding:receipts::return_binding(h.id),
                    item_instance_ids:vec![receipts::command_identity(command,b"lamp-forced-return-item")]};
                result.push(HirelingRecallState{hireling:h,authority:recall,source,content_revision:row.try_get("content_revision")?,ruleset_revision:row.try_get("ruleset_revision")?});
            }
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await
    }

    /// Return even for an offline former owner or a full donor Store Inbox. The native Inbox
    /// owner still enforces its own durable resource ceiling and original command provenance.
    pub async fn recall_hireling_to_lamp(
        &self,authority:&ReconciledCharacterAuthority<'_,'_>,node:&NodeIncarnationProof,
        recall:HirelingRecallRequest,request:HirelingLampReturn,
    )->Result<(),DurabilityError>{
        return_lamp::validate(&request)?;
        if recall.hireling!=request.hireling{return Err(DurabilityError::InvalidStoredState)}
        let mut candidate=self.prepare_hireling_recall_inbox_mint(authority,node,recall.clone(),&request.source,request.mint.clone()).await.map_err(return_lamp::inbox_error)?;
        let mut owner=();
        let result=self.commit_hireling_recall_inbox_mint(authority,node,recall,&mut candidate,&mut owner,
            move|context,_|Box::pin(async move{
                let tx=context.transaction();
                let row=model::snapshot_query!(" FROM game_house_hirelings h \
                    WHERE h.hireling_id=encode($1,'hex')::uuid AND h.active \
                    AND game_house_effective_owner(h.world_id,h.house_key) IS DISTINCT FROM h.owner_character_id FOR UPDATE")
                    .bind(request.hireling.as_slice()).fetch_optional(&mut **tx).await?;
                let Some(row)=row else{return Err(RewardClaimInboxError::SourceRefused)};
                let h=model::decode(&row)?;
                return_lamp::finish_return(tx,&request.source,&h,3).await?;Ok(())
            })).await.map_err(return_lamp::inbox_error)?;
        match result{RewardClaimInboxOutcome::Committed{..}|RewardClaimInboxOutcome::AlreadyCommitted(_)=>Ok(())}
    }
}
