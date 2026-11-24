//! Store blessing delivery uses the same immutable receipt and outbox as other offers.

use super::super::DurabilityRoot;
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::codec::{DeliveryIdentity, DeliveryReceipt, StoreValidationError, uuid_v7};
use super::storage::{
    ReceiptPayload, StoreDurabilityError, account_matches, assert_store_fence, write_receipt,
};
use oteryn_gameplay_content::content::blessing_catalogue::BlessingCatalogue;

pub struct StoreBlessingDelivery {
    pub identity: DeliveryIdentity,
    pub operation_id: [u8; 16],
    pub purchase_request_id: [u8; 16],
    pub blessing_key: String,
}

impl DurabilityRoot {
    pub async fn grant_store_blessing(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        plan: StoreBlessingDelivery,
    ) -> Result<DeliveryReceipt, StoreDurabilityError> {
        plan.identity.validate()?;
        if !uuid_v7(&plan.operation_id) || !uuid_v7(&plan.purchase_request_id) {
            return Err(StoreValidationError::InvalidUuid.into());
        }
        let catalogue = BlessingCatalogue::load()
            .map_err(|_| super::super::DurabilityError::InvalidStoredState)?;
        if !catalogue
            .blessings
            .iter()
            .any(|blessing| blessing.storage_key == plan.blessing_key)
        {
            return Err(StoreValidationError::InvalidText.into());
        }
        if plan.identity.character_id != *fence.character_id.as_bytes()
            || plan.identity.world_id != *fence.runtime_scope.world_id().as_bytes()
        {
            return Err(StoreDurabilityError::AuthorityRejected);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            sqlx::query(
                "SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-delivery:'||encode($1,'hex'),0))",
            ).bind(plan.operation_id.as_slice()).execute(&mut *tx).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_store_fence(&mut tx, &fence, &node, None).await?
                || !account_matches(&mut tx, &fence, &plan.identity.account_id).await?
            {
                return Ok(Err(StoreDurabilityError::AuthorityRejected));
            }
            let entitlement_key = format!("store-blessing:{}", plan.blessing_key);
            let existing: Option<(bool, Vec<u8>)> = sqlx::query_as(
                "SELECT (ROW(account_id, character_id, world_id, fulfilment_unit_id,
                             purchase_request_id, entitlement_key) IS NOT DISTINCT FROM
                         ROW(encode($2,'hex')::uuid, encode($3,'hex')::uuid,
                             encode($4,'hex')::uuid, $5::text, encode($6,'hex')::uuid, $7::text)),
                         receipt
                   FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid",
            )
            .bind(plan.operation_id.as_slice())
            .bind(plan.identity.account_id.as_slice())
            .bind(plan.identity.character_id.as_slice())
            .bind(plan.identity.world_id.as_slice())
            .bind(&plan.identity.fulfilment_unit_id)
            .bind(plan.purchase_request_id.as_slice())
            .bind(&entitlement_key)
            .fetch_optional(&mut *tx)
            .await?;
            let receipt = match existing {
                Some((false, _)) => return Ok(Err(StoreDurabilityError::ConflictingOperation)),
                Some((true, receipt)) => serde_json::from_slice(&receipt)
                    .map_err(|_| super::super::DurabilityError::InvalidStoredState)?,
                None => match write_receipt(
                    &mut tx,
                    &recovery,
                    plan.operation_id,
                    &plan.identity,
                    ReceiptPayload {
                        purchase_request_id: Some(plan.purchase_request_id),
                        entitlement_key: Some(&entitlement_key),
                        ..ReceiptPayload::default()
                    },
                ).await {
                    Ok(receipt) => receipt,
                    Err(error) => return Ok(Err(error)),
                },
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(receipt))
        })).await?
    }
}
