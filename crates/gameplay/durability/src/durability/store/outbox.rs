use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::{DurabilityError, DurabilityRoot};
use super::codec::{DeliveryReceipt, StoreValidationError, uuid_v7};
use super::storage::StoreDurabilityError;
use sqlx::Row;

impl DurabilityRoot {
    /// Read at most 256 pending receipts in decision-time/operation order.
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
                         ORDER BY r.decided_at,r.operation_id LIMIT $1",
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
    /// Mark a known receipt once; repeated or unknown acknowledgements return false.
    pub async fn mark_store_receipt_acknowledged(
        &self,
        id: [u8; 16],
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
                         acknowledged_at=statement_timestamp() \
                         WHERE operation_id=encode($1,'hex')::uuid AND acknowledged_at IS NULL",
                    )
                    .bind(id.as_slice())
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
