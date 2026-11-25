use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::{DurabilityError, DurabilityRoot};
use super::codec::{MAX_ANSWER_BYTES, PurchaseRequest, StoreValidationError, uuid_string, uuid_v7};
use super::purchase_history::StorePurchaseHistoryEntry;
use super::storage::{PurchaseRequestState, StoreDurabilityError};
use sqlx::Row;

impl DurabilityRoot {
    /// Find an accepted ordinary purchase that has no Game delivery receipt.
    pub async fn pending_paid_store_item(
        &self,
        account: [u8; 16],
        offer: String,
        world: [u8; 16],
    ) -> Result<Option<super::PendingStorePurchase>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row: Option<(sqlx::types::Uuid, sqlx::types::Uuid, Vec<u8>)> = sqlx::query_as(
                "SELECT p.purchase_request_id,p.character_id,p.binding_digest
                 FROM game_store_purchase_history h JOIN store_purchase_requests p USING(purchase_request_id)
                 JOIN game_character_roots c ON c.character_id=p.character_id
                 WHERE c.world_id=encode($3,'hex')::uuid AND h.account_id=encode($1,'hex')::uuid AND h.offer_key=$2 AND h.currency='TIBIA_COIN' AND p.state=2
                   AND NOT EXISTS(SELECT 1 FROM store_delivery_receipts r
                       WHERE r.operation_id=(convert_from(p.answer,'UTF8')::jsonb -> 'operation_ids' ->> 0)::uuid)
                   AND NOT EXISTS(SELECT 1 FROM game_store_restore_conflicts conflict
                       WHERE conflict.operation_id=(convert_from(p.answer,'UTF8')::jsonb -> 'operation_ids' ->> 0)::uuid)
                 ORDER BY h.purchased_at_ms,h.purchase_request_id LIMIT 1",
            ).bind(account.as_slice()).bind(offer).bind(world.as_slice()).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(row.map(|(purchase_id, character_id, binding_digest)| super::PendingStorePurchase {
                purchase_id: *purchase_id.as_bytes(),
                character_id: *character_id.as_bytes(),
                world_id: world,
                binding_digest,
            })))
        })).await?
    }

    /// Page legacy accepted journals whose offer is identifiable only by the buyer's quote binding.
    pub async fn pending_legacy_paid_store_items(
        &self,
        account: [u8; 16],
        world: [u8; 16],
        after: Option<[u8; 16]>,
    ) -> Result<Vec<super::PendingStorePurchase>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let rows: Vec<(sqlx::types::Uuid, sqlx::types::Uuid, Vec<u8>)> = sqlx::query_as(
                "SELECT p.purchase_request_id,p.character_id,p.binding_digest
                 FROM store_purchase_requests p JOIN game_character_roots c USING(character_id)
                 WHERE p.account_id=encode($1,'hex')::uuid AND c.world_id=encode($2,'hex')::uuid
                   AND p.state=2 AND game_store_accepted_operation(p.answer) IS NOT NULL
                   AND ($3::uuid IS NULL OR p.purchase_request_id>$3)
                   AND NOT EXISTS(SELECT 1 FROM game_store_purchase_history h
                                  WHERE h.purchase_request_id=p.purchase_request_id)
                   AND NOT EXISTS(SELECT 1 FROM store_delivery_receipts r
                                  WHERE r.operation_id=game_store_accepted_operation(p.answer))
                   AND NOT EXISTS(SELECT 1 FROM game_store_restore_conflicts conflict
                                  WHERE conflict.operation_id=game_store_accepted_operation(p.answer))
                 ORDER BY p.purchase_request_id LIMIT 20",
            ).bind(account.as_slice()).bind(world.as_slice()).bind(after.map(sqlx::types::Uuid::from_bytes))
                .fetch_all(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(rows.into_iter().map(|(purchase_id, character_id, binding_digest)| super::PendingStorePurchase {
                purchase_id: *purchase_id.as_bytes(),
                character_id: *character_id.as_bytes(),
                world_id: world,
                binding_digest,
            }).collect()))
        })).await?
    }

    /// Select unfinished purchases, including legacy journals identified by an exact quote.
    /// Paid journals without a receipt require one of the supplied exact quote bindings.
    /// Removed-offer recovery also limits unanswered journals to those bindings.
    pub async fn pending_store_item_purchases(
        &self,
        account: [u8; 16],
        character: [u8; 16],
        world: [u8; 16],
        legacy_bindings: Vec<Vec<u8>>,
        removed_only: bool,
    ) -> Result<Vec<super::PendingStorePurchase>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let rows: Vec<(sqlx::types::Uuid, Vec<u8>)> = sqlx::query_as(
                "SELECT p.purchase_request_id,p.binding_digest
                 FROM store_purchase_requests p JOIN game_character_roots c USING(character_id)
                 WHERE p.account_id=encode($1,'hex')::uuid AND p.character_id=encode($2,'hex')::uuid
                   AND c.world_id=encode($3,'hex')::uuid
                   AND (
                     (p.state=1 AND (p.binding_digest=ANY($4::bytea[]) OR (NOT $5 AND EXISTS (
                         SELECT 1 FROM game_store_item_reservations r
                         WHERE r.purchase_request_id=p.purchase_request_id AND r.character_id=p.character_id
                     ))))
                     OR (p.state=2 AND p.binding_digest=ANY($4::bytea[])
                         AND convert_from(p.answer,'UTF8')::jsonb ->> 'result' = 'ACCEPTED'
                         AND NOT EXISTS (SELECT 1 FROM store_delivery_receipts r
                             WHERE r.operation_id=(convert_from(p.answer,'UTF8')::jsonb -> 'operation_ids' ->> 0)::uuid)
                         AND NOT EXISTS(SELECT 1 FROM game_store_restore_conflicts conflict
                       WHERE conflict.operation_id=(convert_from(p.answer,'UTF8')::jsonb -> 'operation_ids' ->> 0)::uuid))
                   )
                   AND NOT EXISTS(SELECT 1 FROM game_store_purchase_retries r WHERE r.retry_id=p.purchase_request_id)
                 ORDER BY (p.binding_digest=ANY($4::bytea[])) DESC,p.created_at,p.purchase_request_id LIMIT 20",
            ).bind(account.as_slice()).bind(character.as_slice()).bind(world.as_slice()).bind(legacy_bindings).bind(removed_only)
                .fetch_all(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(rows.into_iter().map(|(purchase_id, binding_digest)| super::PendingStorePurchase {
                purchase_id: *purchase_id.as_bytes(),
                character_id: character,
                world_id: world,
                binding_digest,
            }).collect()))
        })).await?
    }

    pub async fn read_store_purchase_retry(
        &self,
        retry: PurchaseRequest,
    ) -> Result<Option<super::PendingStorePurchase>, StoreDurabilityError> {
        retry.validate()?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row: Option<(sqlx::types::Uuid, sqlx::types::Uuid, sqlx::types::Uuid, Vec<u8>)> = sqlx::query_as(
                "SELECT original.purchase_request_id,original.character_id,retry.world_id,original.binding_digest
                 FROM game_store_purchase_retries retry
                 JOIN store_purchase_requests request ON request.purchase_request_id=retry.retry_id
                 JOIN store_purchase_requests original ON original.purchase_request_id=retry.original_id
                 WHERE retry.retry_id=encode($1,'hex')::uuid
                   AND request.account_id=encode($2,'hex')::uuid AND original.account_id=request.account_id
                   AND request.character_id=encode($3,'hex')::uuid AND request.binding_digest=$4",
            ).bind(retry.purchase_request_id.as_slice()).bind(retry.account_id.as_slice())
                .bind(retry.character_id.as_slice()).bind(retry.binding_digest().as_slice())
                .fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(row.map(|(purchase_id, character_id, world_id, binding_digest)| super::PendingStorePurchase {
                purchase_id: *purchase_id.as_bytes(), character_id: *character_id.as_bytes(),
                world_id: *world_id.as_bytes(), binding_digest,
            })))
        })).await?
    }

    pub async fn record_store_purchase_retry(
        &self,
        retry: PurchaseRequest,
        original: [u8; 16],
        world: [u8; 16],
    ) -> Result<(), StoreDurabilityError> {
        retry.validate()?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let correct = async {
                if record_request(&mut tx, &retry).await? != PurchaseRequestState::Sent {
                    return Err(StoreDurabilityError::ConflictingBinding);
                }
                sqlx::query(
                    "INSERT INTO game_store_purchase_retries(retry_id,original_id,world_id)
                     SELECT encode($1,'hex')::uuid,p.purchase_request_id,encode($4,'hex')::uuid
                     FROM store_purchase_requests p
                     WHERE p.purchase_request_id=encode($2,'hex')::uuid AND p.account_id=encode($3,'hex')::uuid
                       AND p.character_id=encode($5,'hex')::uuid
                     ON CONFLICT(retry_id) DO NOTHING",
                ).bind(retry.purchase_request_id.as_slice()).bind(original.as_slice())
                    .bind(retry.account_id.as_slice()).bind(world.as_slice())
                    .bind(retry.character_id.as_slice()).execute(&mut *tx).await?;
                let matches: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_store_purchase_retries r
                     JOIN store_purchase_requests p ON p.purchase_request_id=r.original_id
                     WHERE r.retry_id=encode($1,'hex')::uuid AND r.original_id=encode($2,'hex')::uuid
                       AND r.world_id=encode($3,'hex')::uuid AND p.character_id=encode($4,'hex')::uuid)",
                ).bind(retry.purchase_request_id.as_slice()).bind(original.as_slice())
                    .bind(world.as_slice()).bind(retry.character_id.as_slice()).fetch_one(&mut *tx).await?;
                if matches { Ok(()) } else { Err(StoreDurabilityError::ConflictingBinding) }
            }.await;
            if correct.is_ok() {
                commit_semantic_transaction(tx, deadline).await?;
            }
            Ok(correct)
        })).await?
    }

    /// Read a terminal reply for the same immutable binding without creating a journal row.
    pub async fn read_store_purchase_answer(
        &self,
        request: PurchaseRequest,
    ) -> Result<Option<Vec<u8>>, StoreDurabilityError> {
        request.validate()?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let answer = sqlx::query_scalar(
                        "SELECT answer FROM store_purchase_requests WHERE state=2 \
                         AND purchase_request_id=encode($1,'hex')::uuid \
                         AND account_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid \
                         AND binding_digest=$4",
                    )
                    .bind(request.purchase_request_id.as_slice())
                    .bind(request.account_id.as_slice())
                    .bind(request.character_id.as_slice())
                    .bind(request.binding_digest().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(answer))
                })
            })
            .await?
    }

    /// Read-only restore gate. Existence permits only the normal retry path, which still
    /// checks the immutable binding before reconciliation; this never creates a request.
    pub async fn has_store_purchase_request(
        &self,
        purchase_request_id: [u8; 16],
        account_id: [u8; 16],
        character_id: [u8; 16],
    ) -> Result<bool, StoreDurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let retained: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM store_purchase_requests \
                     WHERE purchase_request_id=encode($1,'hex')::uuid \
                     AND account_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid)"
                ).bind(purchase_request_id.as_slice()).bind(account_id.as_slice())
                    .bind(character_id.as_slice()).fetch_one(&mut *tx).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(retained))
                })
            })
            .await?
    }

    /// Retain the first immutable purchase binding; retries return its saved answer.
    pub async fn record_purchase_request(
        &self,
        request: PurchaseRequest,
    ) -> Result<PurchaseRequestState, StoreDurabilityError> {
        request.validate()?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let state = match record_request(&mut tx, &request).await {
                        Ok(state) => state,
                        Err(error) => return Ok(Err(error)),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(state))
                })
            })
            .await?
    }

    /// Save exact Platform answer bytes once, under the same purchase binding lock.
    pub async fn record_store_answer(
        &self,
        request: PurchaseRequest,
        answer: Vec<u8>,
    ) -> Result<Vec<u8>, StoreDurabilityError> {
        self.record_store_answer_with_history(request, answer, None)
            .await
    }

    /// Save the Platform answer and its optional immutable Game display row atomically.
    pub async fn record_store_answer_with_history(
        &self,
        request: PurchaseRequest,
        answer: Vec<u8>,
        history: Option<StorePurchaseHistoryEntry>,
    ) -> Result<Vec<u8>, StoreDurabilityError> {
        request.validate()?;
        if answer.is_empty() || answer.len() > MAX_ANSWER_BYTES {
            return Err(StoreValidationError::InvalidBytes.into());
        }
        if let Some(entry) = history.as_ref() {
            let bounded = |text: &str| !text.is_empty() && text.len() <= 128;
            if !uuid_v7(&entry.order_id)
                || !bounded(&entry.offer_key)
                || !bounded(&entry.catalog_revision)
                || !bounded(&entry.currency)
                || entry.price == 0
                || entry.price > i64::MAX as u64
            {
                return Err(StoreValidationError::InvalidBytes.into());
            }
        }
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    match record_request(&mut tx, &request).await {
                        Ok(PurchaseRequestState::Answered(stored)) if stored != answer => {
                            return Ok(Err(StoreDurabilityError::ConflictingBinding));
                        }
                        Ok(PurchaseRequestState::Answered(_)) => {}
                        Ok(PurchaseRequestState::Sent) => {
                            sqlx::query(
                                "UPDATE store_purchase_requests SET state=2, answer=$2 \
                                 WHERE purchase_request_id=encode($1,'hex')::uuid AND state=1",
                            )
                            .bind(request.purchase_request_id.as_slice())
                            .bind(&answer)
                            .execute(&mut *tx)
                            .await?;
                            if let Some(entry) = history.as_ref() {
                                sqlx::query(
                                    "INSERT INTO game_store_purchase_history \
                                     (purchase_request_id,account_id,character_id,order_id,offer_key, \
                                     catalog_revision,price,currency,purchased_at_ms) VALUES \
                                     (encode($1,'hex')::uuid,encode($2,'hex')::uuid, \
                                     encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8, \
                                     floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
                                )
                                .bind(request.purchase_request_id.as_slice())
                                .bind(request.account_id.as_slice())
                                .bind(request.character_id.as_slice())
                                .bind(entry.order_id.as_slice())
                                .bind(&entry.offer_key)
                                .bind(&entry.catalog_revision)
                                .bind(entry.price as i64)
                                .bind(&entry.currency)
                                .execute(&mut *tx)
                                .await?;
                            }
                        }
                        Err(error) => return Ok(Err(error)),
                    }
                    if let Err(error) = super::hireling_reservation::release_refused(&mut tx, &request, &answer).await { return Ok(Err(error)); }
                    if refused(&answer) {
                        for sql in [
                            "DELETE FROM game_store_item_reservations WHERE purchase_request_id=encode($1,'hex')::uuid",
                            "DELETE FROM game_character_store_cosmetics WHERE purchase_id=encode($1,'hex')::uuid AND operation_id IS NULL",
                        ] {
                            sqlx::query(sql).bind(request.purchase_request_id.as_slice()).execute(&mut *tx).await?;
                        }
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(answer))
                })
            })
            .await?
    }
}

pub(super) async fn record_request(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &PurchaseRequest,
) -> Result<PurchaseRequestState, StoreDurabilityError> {
    lock_request(tx, request).await?;
    if let Some(state) = read_request(tx, request).await? {
        return Ok(state);
    }
    insert_request(tx, request).await?;
    Ok(PurchaseRequestState::Sent)
}

pub(super) async fn lock_request(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &PurchaseRequest,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:store-purchase:'||encode($1,'hex'),0))",
    )
    .bind(request.purchase_request_id.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Read an exact binding while the caller holds the purchase advisory lock.
pub(super) async fn read_request(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &PurchaseRequest,
) -> Result<Option<PurchaseRequestState>, StoreDurabilityError> {
    let digest = request.binding_digest();
    if let Some(row) = sqlx::query(
        "SELECT account_id::text,character_id::text,binding_digest,state,answer \
         FROM store_purchase_requests WHERE purchase_request_id=encode($1,'hex')::uuid",
    )
    .bind(request.purchase_request_id.as_slice())
    .fetch_optional(&mut **tx)
    .await?
    {
        if row.try_get::<String, _>("account_id")? != uuid_string(&request.account_id)
            || row.try_get::<String, _>("character_id")? != uuid_string(&request.character_id)
            || row.try_get::<Vec<u8>, _>("binding_digest")? != digest
        {
            return Err(StoreDurabilityError::ConflictingBinding);
        }
        return match row.try_get::<i16, _>("state")? {
            1 => Ok(Some(PurchaseRequestState::Sent)),
            2 => Ok(Some(PurchaseRequestState::Answered(row.try_get("answer")?))),
            _ => Err(DurabilityError::InvalidStoredState.into()),
        };
    }
    Ok(None)
}

/// Insert only after checking the binding under the purchase advisory lock.
pub(super) async fn insert_request(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &PurchaseRequest,
) -> Result<(), StoreDurabilityError> {
    let digest = request.binding_digest();
    sqlx::query(
        "INSERT INTO store_purchase_requests \
         (purchase_request_id,account_id,character_id,binding_digest,state) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,1)",
    )
    .bind(request.purchase_request_id.as_slice())
    .bind(request.account_id.as_slice())
    .bind(request.character_id.as_slice())
    .bind(digest.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(super) fn refused(answer: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(answer).is_ok_and(|value| {
        matches!(value["result"].as_str(), Some(
            "STALE_QUOTE" | "OFFER_UNAVAILABLE" | "PURCHASE_DISABLED" |
            "INSUFFICIENT_BALANCE" | "ACCOUNT_RESTRICTED"
        ))
    })
}
