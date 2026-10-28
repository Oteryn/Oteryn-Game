use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::{DurabilityError, DurabilityRoot};
use super::codec::{MAX_ANSWER_BYTES, PurchaseRequest, StoreValidationError, uuid_string};
use super::storage::{PurchaseRequestState, StoreDurabilityError};
use sqlx::Row;

impl DurabilityRoot {
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
        request.validate()?;
        if answer.is_empty() || answer.len() > MAX_ANSWER_BYTES {
            return Err(StoreValidationError::InvalidBytes.into());
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
                        }
                        Err(error) => return Ok(Err(error)),
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(answer))
                })
            })
            .await?
    }
}

async fn record_request(
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
