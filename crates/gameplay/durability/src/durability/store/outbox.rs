use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::{DurabilityError, DurabilityRoot};
use super::codec::{DeliveryReceipt, StoreValidationError, uuid_v7};
use super::storage::StoreDurabilityError;
use sqlx::Row;

impl DurabilityRoot {
    /// Read at most 256 pending receipts, giving untried deliveries priority.
    pub async fn list_unacknowledged_store_receipts(
        &self,
        limit: u16,
    ) -> Result<Vec<DeliveryReceipt>, StoreDurabilityError> {
        if limit == 0 || limit > 256 {
            return Err(StoreValidationError::InvalidQuantity.into());
        }
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let rows = sqlx::query(
                        "SELECT r.receipt FROM store_receipt_outbox o \
                         JOIN store_delivery_receipts r USING(operation_id) \
                         WHERE o.acknowledged_at IS NULL \
                         ORDER BY o.publish_attempts,r.decided_at,r.operation_id LIMIT $1",
                    )
                    .bind(i64::from(limit))
                    .fetch_all(&mut *tx)
                    .await?;
                    let mut receipts = Vec::with_capacity(rows.len());
                    for row in rows {
                        receipts.push(
                            serde_json::from_slice(&row.try_get::<Vec<u8>, _>("receipt")?)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                        );
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(receipts))
                })
            })
            .await?
    }
    /// Read the reconciled generation installed by the Game recovery owner.
    pub async fn store_publication_generation(&self) -> Result<Option<String>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let generation = sqlx::query_scalar(
                "SELECT CASE WHEN reconciled_at IS NOT NULL THEN recovery_generation::text END
                 FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1",
            ).fetch_optional(&mut *tx).await?.flatten();
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(generation))
        })).await?
    }

    /// A committed native receipt and its realized unit are the proof for a Platform CLAIM.
    pub async fn read_store_claim_source(
        &self,
        delivery_id: [u8; 16],
    ) -> Result<Option<(DeliveryReceipt, [u8; 16])>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row: Option<(Vec<u8>, sqlx::types::Uuid)> = sqlx::query_as(
                "SELECT receipt, COALESCE(item_instance_id,operation_id)
                 FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid",
            ).bind(delivery_id.as_slice()).fetch_optional(&mut *tx).await?;
            let source = row.map(|(bytes, unit)| {
                serde_json::from_slice(&bytes).map(|receipt| (receipt, *unit.as_bytes()))
                    .map_err(|_| DurabilityError::InvalidStoredState)
            }).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(source))
        })).await?
    }

    /// Mark a known receipt once; repeated or unknown acknowledgements return false.
    pub async fn mark_store_receipt_acknowledged(
        &self,
        id: [u8; 16],
    ) -> Result<bool, StoreDurabilityError> {
        self.record_store_receipt_attempt(id, true).await
    }

    /// Keep a failed publication pending and let other receipts get their turn.
    pub async fn record_store_receipt_attempt(
        &self,
        id: [u8; 16],
        acknowledged: bool,
    ) -> Result<bool, StoreDurabilityError> {
        if !uuid_v7(&id) {
            return Err(StoreValidationError::InvalidUuid.into());
        }
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let changed = sqlx::query(
                        "UPDATE store_receipt_outbox SET publish_attempts=publish_attempts+1, \
                         acknowledged_at=CASE WHEN $2 THEN statement_timestamp() END \
                         WHERE operation_id=encode($1,'hex')::uuid AND acknowledged_at IS NULL",
                    )
                    .bind(id.as_slice())
                    .bind(acknowledged)
                    .execute(&mut *tx)
                    .await?
                    .rows_affected()
                        == 1;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(changed))
                })
            })
            .await?
    }
}
