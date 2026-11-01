//! Non-item Hireling effects extend the existing Store receipt/outbox owner.
use super::super::character_authority::{ReconciledCharacterAuthority,assert_recovery_fence};
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::super::db::{begin_semantic_transaction,commit_semantic_transaction,lock_admission_relations};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::DurabilityRoot;
use super::codec::{DeliveryReceipt,StoreValidationError,uuid_v7};
use super::hireling_model::{HirelingServiceDeliveryPlan,HirelingStoreAction};
use super::storage::{StoreDurabilityError,account_matches,lookup_delivery,write_receipt,assert_store_fence};

impl DurabilityRoot {
    pub async fn apply_store_hireling_service(
        &self, authority:&ReconciledCharacterAuthority<'_, '_>, node:&NodeIncarnationProof,
        operation_id:[u8;16], plan:HirelingServiceDeliveryPlan, fence:CurrentCharacterGameplayFence,
    ) -> Result<DeliveryReceipt,StoreDurabilityError> {
        plan.identity.validate()?;
        plan.purchase.validate()?;
        if !uuid_v7(&operation_id) { return Err(StoreValidationError::InvalidUuid.into()); }
        if matches!(plan.purchase.action,HirelingStoreAction::Hire{..}) { return Err(StoreDurabilityError::ConflictingOperation); }
        if plan.identity.character_id != *fence.character_id.as_bytes() || plan.command.game_session_id()!=fence.game_session_id { return Err(StoreDurabilityError::AuthorityRejected); }
        let world_id=fence.runtime_scope.world_id();
        if plan.identity.world_id!=*world_id.as_bytes() {return Err(StoreDurabilityError::AuthorityRejected)}
        let recovery=authority.record_for(self).map_err(|_|StoreDurabilityError::AuthorityRejected)?;
        let node=node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-delivery:'||encode($1,'hex'),0))")
                .bind(operation_id.as_slice()).execute(&mut *tx).await?;
            match lookup_delivery(&mut tx,operation_id,&plan.identity,Some(&plan.purchase)).await? {
                Ok(Some(receipt))=>{commit_semantic_transaction(tx,deadline).await?;return Ok(Ok(receipt))}
                Ok(None)=>{}
                Err(error)=>return Ok(Err(error)),
            }
            lock_admission_relations(&mut tx).await?;
            if !assert_store_fence(&mut tx,&fence,&node,Some(plan.purchase.origin_channel)).await? || !account_matches(&mut tx,&fence,&plan.identity.account_id).await? {return Ok(Err(StoreDurabilityError::AuthorityRejected))}
            if let Err(error)=super::hireling_reservation::apply_effect(&mut tx,plan.identity.character_id,operation_id,None,&plan.purchase).await {return Ok(Err(error))}
            let receipt=match write_receipt(&mut tx,&recovery,operation_id,&plan.identity,None,Some(&plan.purchase),None).await {Ok(v)=>v,Err(error)=>return Ok(Err(error))};
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(receipt))
        })).await?
    }
}
