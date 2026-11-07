use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::{DurabilityError, DurabilityRoot};
use super::codec::{MAX_ANSWER_BYTES, PurchaseRequest, StoreValidationError, uuid_string, uuid_v7};
use super::purchase_history::StorePurchaseHistoryEntry;
use super::storage::{PurchaseRequestState, StoreDurabilityError};
use sqlx::Row;

impl DurabilityRoot {
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
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:store-purchase:'||encode($1,'hex'),0))",
    )
    .bind(request.purchase_request_id.as_slice())
    .execute(&mut **tx)
    .await?;
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
            1 => Ok(PurchaseRequestState::Sent),
            2 => Ok(PurchaseRequestState::Answered(row.try_get("answer")?)),
            _ => Err(DurabilityError::InvalidStoredState.into()),
        };
    }
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
    Ok(PurchaseRequestState::Sent)
}
