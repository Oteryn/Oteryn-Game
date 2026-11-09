//! Permanent Charm Expansion Store reservation and delivery.
use super::super::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence},
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    runtime_scope_assignment::NodeIncarnationProof,
};
use super::codec::{DeliveryIdentity, DeliveryReceipt, encode_receipt, uuid_string, uuid_v7};
use super::{PurchaseRequest, StoreDurabilityError};

impl DurabilityRoot {
    pub async fn reserve_charm_expansion_purchase(
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
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(StoreDurabilityError::AuthorityRejected));
            }
            let matches: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM store_purchase_requests p JOIN game_character_roots c USING(character_id) WHERE p.purchase_request_id=encode($1,'hex')::uuid AND p.character_id=encode($2,'hex')::uuid AND p.account_id=encode($3,'hex')::uuid AND c.account_id=p.account_id AND c.world_id=encode($4,'hex')::uuid AND p.binding_digest=$5)")
                .bind(request.purchase_request_id.as_slice()).bind(request.character_id.as_slice())
                .bind(request.account_id.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(request.binding_digest().as_slice()).fetch_one(&mut *tx).await?;
            if !matches { return Ok(Err(StoreDurabilityError::ConflictingBinding)); }
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:charm-expansion-purchase:'||encode($1,'hex'),0))")
                .bind(request.character_id.as_slice()).execute(&mut *tx).await?;
            let pending: Option<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(purchase_id) FROM game_charm_expansion_purchases WHERE character_id=encode($1,'hex')::uuid")
                .bind(request.character_id.as_slice()).fetch_optional(&mut *tx).await?;
            let allowed = match pending {
                Some(id) => id == request.purchase_request_id,
                None => {
                    let owned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_charm_expansions WHERE character_id=encode($1,'hex')::uuid)")
                        .bind(request.character_id.as_slice()).fetch_one(&mut *tx).await?;
                    if owned { false } else {
                        sqlx::query("INSERT INTO game_charm_expansion_purchases(character_id,purchase_id,world_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
                            .bind(request.character_id.as_slice()).bind(request.purchase_request_id.as_slice())
                            .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).execute(&mut *tx).await?;
                        true
                    }
                }
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(allowed))
        })).await?
    }

    pub async fn grant_charm_expansion(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        operation_id: [u8; 16],
        identity: DeliveryIdentity,
    ) -> Result<DeliveryReceipt, StoreDurabilityError> {
        if !uuid_v7(&operation_id) {
            return Err(StoreDurabilityError::ConflictingOperation);
        }
        identity.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err()
                || identity.character_id != *fence.character_id.as_bytes()
                || identity.world_id != *fence.runtime_scope.world_id().as_bytes() {
                return Ok(Err(StoreDurabilityError::AuthorityRejected));
            }
            let reserved: Option<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(p.purchase_id) FROM game_charm_expansion_purchases p JOIN game_character_roots c USING(character_id) WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid AND c.account_id=encode($3,'hex')::uuid")
                .bind(identity.character_id.as_slice()).bind(identity.world_id.as_slice()).bind(identity.account_id.as_slice()).fetch_one(&mut *tx).await?;
            let Some(purchase_id) = reserved else { return Ok(Err(StoreDurabilityError::ConflictingOperation)); };
            let Ok(purchase_uuid): Result<&[u8; 16], _> = purchase_id.as_slice().try_into() else { return Ok(Err(StoreDurabilityError::ConflictingOperation)); };
            sqlx::query("INSERT INTO game_character_charm_expansions(character_id) VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING")
                .bind(identity.character_id.as_slice()).execute(&mut *tx).await?;
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint").fetch_one(&mut *tx).await?;
            let receipt = DeliveryReceipt { operation_id: uuid_string(&operation_id), account_id: uuid_string(&identity.account_id), character_id: uuid_string(&identity.character_id), world_id: uuid_string(&identity.world_id), fulfilment_unit_id: identity.fulfilment_unit_id, result: "COMMITTED".into(), recovery_generation: recovery.recovery_generation.to_string(), decided_at: now.to_string() };
            let bytes = match encode_receipt(&receipt) {
                Ok(bytes) => bytes,
                Err(_) => return Ok(Err(StoreDurabilityError::ConflictingOperation)),
            };
            sqlx::query("INSERT INTO store_delivery_receipts(operation_id,account_id,character_id,world_id,fulfilment_unit_id,result,recovery_generation,decided_at,receipt,item_instance_id,entitlement_key,purchase_request_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,'COMMITTED',$6::text::numeric,$7,$8,NULL,'charm-expansion',$9::uuid) ON CONFLICT (operation_id) DO NOTHING")
                .bind(operation_id.as_slice()).bind(identity.account_id.as_slice()).bind(identity.character_id.as_slice()).bind(identity.world_id.as_slice()).bind(&receipt.fulfilment_unit_id).bind(recovery.recovery_generation.to_string()).bind(now).bind(bytes).bind(uuid_string(purchase_uuid)).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO store_receipt_outbox(operation_id) VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING").bind(operation_id.as_slice()).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(receipt))
        })).await?
    }

    pub async fn release_refused_charm_expansion_purchase(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        request: PurchaseRequest,
    ) -> Result<(), StoreDurabilityError> {
        request.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let answer: Option<Vec<u8>> = sqlx::query_scalar("SELECT answer FROM store_purchase_requests WHERE purchase_request_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND account_id=encode($3,'hex')::uuid AND binding_digest=$4 AND state=2")
                .bind(request.purchase_request_id.as_slice()).bind(request.character_id.as_slice())
                .bind(request.account_id.as_slice()).bind(request.binding_digest().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(answer) = answer else { return Ok(Err(StoreDurabilityError::ConflictingBinding)); };
            let valid = serde_json::from_slice::<serde_json::Value>(&answer).ok().is_some_and(|value| {
                value.get("order_id").is_some_and(serde_json::Value::is_null)
                    && value.get("operation_ids").and_then(serde_json::Value::as_array).is_some_and(Vec::is_empty)
                    && matches!(value.get("result").and_then(serde_json::Value::as_str), Some("STALE_QUOTE" | "OFFER_UNAVAILABLE" | "PURCHASE_DISABLED" | "INSUFFICIENT_BALANCE" | "ACCOUNT_RESTRICTED"))
            });
            if !valid { return Ok(Err(StoreDurabilityError::ConflictingBinding)); }
            sqlx::query("DELETE FROM game_charm_expansion_purchases WHERE character_id=encode($1,'hex')::uuid AND purchase_id=encode($2,'hex')::uuid")
                .bind(request.character_id.as_slice()).bind(request.purchase_request_id.as_slice()).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }
}
