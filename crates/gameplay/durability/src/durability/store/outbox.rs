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
    pub async fn store_publication_generation(
        &self,
    ) -> Result<Option<String>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let generation = sqlx::query_scalar(
                "SELECT CASE WHEN reconciled_at IS NOT NULL THEN recovery_generation::text END
                 FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1",
            ).fetch_optional(&mut *tx).await?.flatten();
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(generation))
                })
            })
            .await?
    }

    /// Register a Platform-owned Premium Time grant without changing the premium snapshot.
    pub async fn record_premium_store_registration(
        &self,
        operation: [u8; 16],
        binding_digest: String,
        restore_generation: Option<u64>,
    ) -> Result<Option<(String, i64)>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:character-recovery-admission', 0))")
                .execute(&mut *tx).await?;
            let current: Option<String> = sqlx::query_scalar(
                "SELECT CASE WHEN reconciled_at IS NOT NULL THEN recovery_generation::text END
                 FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1",
            ).fetch_optional(&mut *tx).await?.flatten();
            let Some(current) = current else { return Ok(Ok(None)); };
            if restore_generation.is_some_and(|generation| current != generation.to_string()) {
                return Ok(Ok(None));
            }
            sqlx::query(
                "INSERT INTO game_store_premium_registrations(operation_id,binding_digest,recovery_generation,decided_at)
                 VALUES (encode($1,'hex')::uuid,decode($2,'hex'),$3::text::numeric,
                         floor(extract(epoch FROM statement_timestamp()))::bigint)
                 ON CONFLICT DO NOTHING",
            ).bind(operation.as_slice()).bind(&binding_digest).bind(current)
                .execute(&mut *tx).await?;
            let registration = sqlx::query_as(
                "SELECT recovery_generation::text,decided_at FROM game_store_premium_registrations
                 WHERE operation_id=encode($1,'hex')::uuid AND binding_digest=decode($2,'hex')",
            ).bind(operation.as_slice()).bind(binding_digest).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(registration))
        })).await?
    }

    /// Preserve a lost decision before reporting it to Platform for manual review.
    pub async fn record_store_restore_conflict(
        &self,
        operation: [u8; 16],
        generation: u64,
        delivery: Option<[u8; 16]>,
    ) -> Result<Option<i64>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let decided_at = sqlx::query_scalar(
                "WITH latest AS (
                     SELECT recovery_generation, predecessor_generation, reconciled_at
                     FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1
                 ), restored AS (
                     SELECT reconciled_at FROM latest
                     WHERE recovery_generation=$2::text::numeric AND predecessor_generation>0
                 ), marked AS (
                     INSERT INTO game_store_restore_conflicts(operation_id,delivery_operation_id)
                     SELECT encode($1,'hex')::uuid,encode($3,'hex')::uuid FROM restored
                     ON CONFLICT DO NOTHING
                 ) SELECT reconciled_at / 1000 FROM restored",
            ).bind(operation.as_slice()).bind(generation.to_string()).bind(delivery.as_ref().map(|id| id.as_slice())).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(decided_at))
        })).await?
    }

    pub async fn has_store_restore_conflict(
        &self,
        operation: [u8; 16],
    ) -> Result<bool, StoreDurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let marked = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM game_store_restore_conflicts
                 WHERE operation_id=encode($1,'hex')::uuid)",
                    )
                    .bind(operation.as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(marked))
                })
            })
            .await?
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

    /// A committed Inbox retrieval or lamp placement is the proof for a Platform CLAIM.
    pub async fn read_store_claim_source(
        &self,
        delivery_id: [u8; 16],
    ) -> Result<Option<StoreClaimSource>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let source = claim_source_in_transaction(&mut tx, delivery_id).await?;
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
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
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
                })
            })
            .await?
    }

    /// Find the exact Player-bound Store item named by the entitlement.
    pub async fn read_store_claim_operation(
        &self,
        account: [u8; 16],
        character: [u8; 16],
        world: [u8; 16],
        entitlement: String,
    ) -> Result<Option<([u8; 16], [u8; 16], [u8; 16])>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let source: Option<(sqlx::types::Uuid, sqlx::types::Uuid, sqlx::types::Uuid)> =
                        sqlx::query_as(
                            "SELECT r.operation_id,c.claim_operation_id,r.item_instance_id
                 FROM store_delivery_receipts r JOIN game_store_claim_operations c
                   ON c.delivery_operation_id=r.operation_id
                 WHERE r.account_id=encode($1,'hex')::uuid AND r.character_id=encode($2,'hex')::uuid
                   AND r.world_id=encode($3,'hex')::uuid AND r.fulfilment_unit_id=$4
                   AND r.item_instance_id IS NOT NULL
                 ORDER BY r.operation_id LIMIT 1",
                        )
                        .bind(account.as_slice())
                        .bind(character.as_slice())
                        .bind(world.as_slice())
                        .bind(entitlement)
                        .fetch_optional(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(source.map(|(delivery, claim, item)| {
                        (*delivery.as_bytes(), *claim.as_bytes(), *item.as_bytes())
                    })))
                })
            })
            .await?
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


pub type StoreClaimSource = (DeliveryReceipt, [u8; 16]);

pub(in crate::durability) async fn claim_source_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    delivery_id: [u8; 16],
) -> Result<Option<StoreClaimSource>, DurabilityError> {
    let row: Option<(Vec<u8>, sqlx::types::Uuid, i64, String)> = sqlx::query_as(
        "SELECT d.receipt,d.item_instance_id,floor(extract(epoch FROM proof.decided_at))::bigint,
                proof.recovery_generation::text
         FROM store_delivery_receipts d
         CROSS JOIN LATERAL (
             SELECT m.created_at AS decided_at,m.transaction_id,
                    COALESCE(m.inbox_recovery_generation,d.recovery_generation) AS recovery_generation
             FROM game_item_loop_move_lines l
             JOIN game_item_loop_move_receipts m USING(transaction_id)
             WHERE l.item_instance_id=d.item_instance_id AND l.from_kind=6 AND l.to_kind=1
               AND m.character_id=d.character_id AND m.world_id=d.world_id
             UNION ALL
             SELECT to_timestamp(u.occurred_at::double precision / 1000),u.transaction_id,
                    COALESCE(t.placement_recovery_generation,d.recovery_generation)
             FROM game_item_use_records u
             JOIN game_hireling_lamp_transitions t USING(transaction_id)
             WHERE u.source_item_instance_id=d.item_instance_id AND u.source_kind=2 AND u.cause_kind=470
               AND u.character_id=d.character_id AND u.world_id=d.world_id
         ) proof
         WHERE d.operation_id=encode($1,'hex')::uuid
         ORDER BY proof.decided_at,proof.transaction_id LIMIT 1",
    ).bind(delivery_id.as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|(bytes, unit, decided_at, generation)| {
        let mut receipt: DeliveryReceipt =
            serde_json::from_slice(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
        receipt.decided_at = decided_at.to_string();
        receipt.recovery_generation = generation;
        Ok::<_, DurabilityError>((receipt, *unit.as_bytes()))
    }).transpose()
 }
