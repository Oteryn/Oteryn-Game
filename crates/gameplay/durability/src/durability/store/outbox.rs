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

    /// Read committed delivery evidence even after the outbox was acknowledged.
    pub async fn read_retained_store_delivery(
        &self,
        operation: [u8; 16],
    ) -> Result<Option<DeliveryReceipt>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let bytes: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT receipt FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid",
            ).bind(operation.as_slice()).fetch_optional(&mut *tx).await?;
            let receipt = bytes.map(|bytes| serde_json::from_slice(&bytes)
                .map_err(|_| DurabilityError::InvalidStoredState)).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(receipt))
        })).await?
    }

    /// A committed native receipt and its realized unit are the proof for a Platform CLAIM.
    pub async fn read_store_claim_source(
        &self,
        delivery_id: [u8; 16],
    ) -> Result<Option<(DeliveryReceipt, [u8; 16])>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row: Option<(Vec<u8>, sqlx::types::Uuid, i64)> = sqlx::query_as(
                "SELECT d.receipt,d.item_instance_id,floor(extract(epoch FROM m.created_at))::bigint
                 FROM store_delivery_receipts d
                 JOIN game_item_loop_move_lines l ON l.item_instance_id=d.item_instance_id
                 JOIN game_item_loop_move_receipts m ON m.transaction_id=l.transaction_id
                 WHERE d.operation_id=encode($1,'hex')::uuid AND l.from_kind=6 AND l.to_kind=1
                   AND m.character_id=d.character_id AND m.world_id=d.world_id
                 ORDER BY m.created_at,m.transaction_id LIMIT 1",
            ).bind(delivery_id.as_slice()).fetch_optional(&mut *tx).await?;
            let source = row.map(|(bytes, unit, decided_at)| {
                let mut receipt: DeliveryReceipt =
                    serde_json::from_slice(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
                receipt.decided_at = decided_at.to_string();
                Ok::<_, DurabilityError>((receipt, *unit.as_bytes()))
            }).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(source))
        })).await?
    }

    /// Preserve the Platform-issued claim identity for player commands and replays.
    pub async fn remember_store_claim_operation(
        &self,
        delivery: [u8; 16],
        claim: [u8; 16],
        account: String,
        entitlement: String,
    ) -> Result<(), StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            sqlx::query(
                "INSERT INTO game_store_claim_operations (delivery_operation_id,claim_operation_id)
                 SELECT operation_id,encode($2,'hex')::uuid FROM store_delivery_receipts
                 WHERE operation_id=encode($1,'hex')::uuid AND account_id=$3::text::uuid
                   AND fulfilment_unit_id=$4 AND item_instance_id IS NOT NULL
                 ON CONFLICT DO NOTHING",
            ).bind(delivery.as_slice()).bind(claim.as_slice()).bind(account).bind(entitlement)
                .execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }

    /// Find the exact Player-bound Store item named by the entitlement.
    pub async fn read_store_claim_operation(
        &self,
        account: [u8; 16],
        character: [u8; 16],
        world: [u8; 16],
        entitlement: String,
    ) -> Result<Option<([u8; 16], [u8; 16], [u8; 16])>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let source: Option<(sqlx::types::Uuid, sqlx::types::Uuid, sqlx::types::Uuid)> = sqlx::query_as(
                "SELECT r.operation_id,c.claim_operation_id,r.item_instance_id
                 FROM store_delivery_receipts r JOIN game_store_claim_operations c
                   ON c.delivery_operation_id=r.operation_id
                 WHERE r.account_id=encode($1,'hex')::uuid AND r.character_id=encode($2,'hex')::uuid
                   AND r.world_id=encode($3,'hex')::uuid AND r.fulfilment_unit_id=$4
                   AND r.item_instance_id IS NOT NULL
                 ORDER BY r.operation_id LIMIT 1",
            ).bind(account.as_slice()).bind(character.as_slice()).bind(world.as_slice()).bind(entitlement)
                .fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(source.map(|(delivery, claim, item)| (*delivery.as_bytes(), *claim.as_bytes(), *item.as_bytes()))))
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
