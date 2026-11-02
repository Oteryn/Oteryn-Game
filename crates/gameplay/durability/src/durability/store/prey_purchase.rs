//! Prevent a permanent Prey entitlement from opening two concurrent Store debits.
use super::super::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence},
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    runtime_scope_assignment::NodeIncarnationProof,
};
use super::{PurchaseRequest, PurchaseRequestState, StoreDurabilityError};
use sqlx::Row;

impl DurabilityRoot {
    /// A catalogue retune cannot strand an already reserved permanent purchase. This lookup
    /// never records or reserves a new request: only its exact retained binding may resume.
    /// A reservation proves that the request passed the original enabled catalogue before
    /// Platform could debit it. A definitive refusal can replay after releasing that proof.
    pub async fn read_prey_expansion_purchase(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: PurchaseRequest,
    ) -> Result<Option<PurchaseRequestState>, StoreDurabilityError> {
        request.validate()?;
        if request.character_id != *fence.character_id.as_bytes() {
            return Err(StoreDurabilityError::AuthorityRejected);
        }
        let recovery = authority.record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err() {
                    return Ok(Err(StoreDurabilityError::AuthorityRejected));
                }
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:prey-expansion-purchase:'||encode($1,'hex'),0))")
                    .bind(request.character_id.as_slice()).execute(&mut *tx).await?;
                let row = sqlx::query(
                    "SELECT uuid_send(p.account_id) AS account_id,uuid_send(p.character_id) AS character_id,uuid_send(c.account_id) AS root_account,uuid_send(c.world_id) AS world_id,p.binding_digest,p.state,p.answer,EXISTS(SELECT 1 FROM game_prey_expansion_purchases r WHERE r.purchase_id=p.purchase_request_id AND r.character_id=p.character_id AND r.world_id=c.world_id) AS reserved FROM store_purchase_requests p JOIN game_character_roots c USING(character_id) WHERE p.purchase_request_id=encode($1,'hex')::uuid"
                ).bind(request.purchase_request_id.as_slice()).fetch_optional(&mut *tx).await?;
                let Some(row) = row else {
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(Ok(None));
                };
                if row.try_get::<Vec<u8>,_>("account_id")? != request.account_id
                    || row.try_get::<Vec<u8>,_>("root_account")? != request.account_id
                    || row.try_get::<Vec<u8>,_>("character_id")? != request.character_id
                    || row.try_get::<Vec<u8>,_>("world_id")? != fence.runtime_scope.world_id().as_bytes().as_slice()
                    || row.try_get::<Vec<u8>,_>("binding_digest")? != request.binding_digest()
                {
                    return Ok(Err(StoreDurabilityError::ConflictingBinding));
                }
                let reserved: bool = row.try_get("reserved")?;
                let state = match row.try_get::<i16,_>("state")? {
                    1 => reserved.then_some(PurchaseRequestState::Sent),
                    2 => {
                        let answer: Vec<u8> = row.try_get("answer")?;
                        if !reserved && !definitive_refusal(&answer) {
                            return Ok(Err(StoreDurabilityError::ConflictingBinding));
                        }
                        Some(PurchaseRequestState::Answered(answer))
                    }
                    _ => return Ok(Err(StoreDurabilityError::ConflictingBinding)),
                };
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(state))
            })
        }).await?
    }

    /// The immutable Store request is retained first. The same pending id may retry;
    /// another request cannot debit while delivery or a Platform answer is unresolved.
    pub async fn reserve_prey_expansion_purchase(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: PurchaseRequest,
    ) -> Result<bool, StoreDurabilityError> {
        request.validate()?;
        if request.character_id != *fence.character_id.as_bytes() {
            return Err(StoreDurabilityError::AuthorityRejected);
        }
        let recovery = authority.record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err() {
                    return Ok(Err(StoreDurabilityError::AuthorityRejected));
                }
                let matches: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM store_purchase_requests p JOIN game_character_roots c USING(character_id) WHERE p.purchase_request_id=encode($1,'hex')::uuid AND p.character_id=encode($2,'hex')::uuid AND p.account_id=encode($3,'hex')::uuid AND c.account_id=p.account_id AND c.world_id=encode($4,'hex')::uuid AND p.binding_digest=$5)"
                ).bind(request.purchase_request_id.as_slice()).bind(request.character_id.as_slice())
                .bind(request.account_id.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(request.binding_digest().as_slice()).fetch_one(&mut *tx).await?;
                if !matches { return Ok(Err(StoreDurabilityError::ConflictingBinding)); }
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:prey-expansion-purchase:'||encode($1,'hex'),0))")
                    .bind(request.character_id.as_slice()).execute(&mut *tx).await?;
                let pending: Option<Vec<u8>> = sqlx::query_scalar(
                    "SELECT uuid_send(purchase_id) FROM game_prey_expansion_purchases WHERE character_id=encode($1,'hex')::uuid"
                ).bind(request.character_id.as_slice()).fetch_optional(&mut *tx).await?;
                let allowed = match pending {
                    Some(id) => id == request.purchase_request_id,
                    None => {
                        let owned: bool = sqlx::query_scalar(
                            "SELECT EXISTS(SELECT 1 FROM game_character_prey_expansions WHERE character_id=encode($1,'hex')::uuid)"
                        ).bind(request.character_id.as_slice()).fetch_one(&mut *tx).await?;
                        if owned { false } else {
                            sqlx::query("INSERT INTO game_prey_expansion_purchases(character_id,purchase_id,world_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
                                .bind(request.character_id.as_slice()).bind(request.purchase_request_id.as_slice())
                                .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                                .execute(&mut *tx).await?;
                            true
                        }
                    }
                };
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(allowed))
            })
        }).await?
    }

    /// A transport timeout can never release the reservation. Only an already retained
    /// definitive Platform refusal proves that this request cannot have a later debit.
    pub async fn release_refused_prey_expansion_purchase(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        request: PurchaseRequest,
    ) -> Result<(), StoreDurabilityError> {
        request.validate()?;
        let recovery = authority.record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:prey-expansion-purchase:'||encode($1,'hex'),0))")
                    .bind(request.character_id.as_slice()).execute(&mut *tx).await?;
                let row = sqlx::query(
                    "SELECT answer,binding_digest FROM store_purchase_requests WHERE purchase_request_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND account_id=encode($3,'hex')::uuid AND state=2"
                ).bind(request.purchase_request_id.as_slice()).bind(request.character_id.as_slice())
                .bind(request.account_id.as_slice()).fetch_optional(&mut *tx).await?;
                let Some(row) = row else { return Ok(Err(StoreDurabilityError::ConflictingBinding)); };
                if row.try_get::<Vec<u8>,_>("binding_digest")? != request.binding_digest() {
                    return Ok(Err(StoreDurabilityError::ConflictingBinding));
                }
                let answer: Vec<u8> = row.try_get("answer")?;
                if !definitive_refusal(&answer) {
                    return Ok(Err(StoreDurabilityError::ConflictingBinding));
                }
                sqlx::query("DELETE FROM game_prey_expansion_purchases WHERE character_id=encode($1,'hex')::uuid AND purchase_id=encode($2,'hex')::uuid")
                    .bind(request.character_id.as_slice()).bind(request.purchase_request_id.as_slice())
                    .execute(&mut *tx).await?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(()))
            })
        }).await?
    }
}

fn definitive_refusal(bytes: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else { return false; };
    value.get("order_id").is_some_and(serde_json::Value::is_null)
        && value.get("operation_ids").and_then(serde_json::Value::as_array).is_some_and(Vec::is_empty)
        && matches!(value.get("result").and_then(serde_json::Value::as_str),
            Some("STALE_QUOTE" | "OFFER_UNAVAILABLE" | "PURCHASE_DISABLED" | "INSUFFICIENT_BALANCE" | "ACCOUNT_RESTRICTED"))
}

#[cfg(test)]
mod tests {
    use super::definitive_refusal;

    #[test]
    fn prey_purchase_reservation_is_never_released_on_unknown_or_accepted_answer() {
        for result in ["STALE_QUOTE", "OFFER_UNAVAILABLE", "PURCHASE_DISABLED", "INSUFFICIENT_BALANCE", "ACCOUNT_RESTRICTED"] {
            assert!(definitive_refusal(serde_json::json!({"result":result,"order_id":null,"operation_ids":[]}).to_string().as_bytes()));
        }
        for value in [
            serde_json::json!({"result":"ACCEPTED","order_id":"order","operation_ids":["operation"]}),
            serde_json::json!({"result":"TIMEOUT","order_id":null,"operation_ids":[]}),
            serde_json::json!({"result":"INSUFFICIENT_BALANCE","order_id":"order","operation_ids":[]}),
        ] { assert!(!definitive_refusal(value.to_string().as_bytes())); }
        assert!(!definitive_refusal(b""));
    }
}
