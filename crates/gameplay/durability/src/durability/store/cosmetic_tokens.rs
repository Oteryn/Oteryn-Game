//! Character-bound Store cosmetics and atomic earned-token spending.
use super::super::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence},
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    runtime_scope_assignment::NodeIncarnationProof,
};
use super::{
    DeliveryIdentity, DeliveryReceipt, PurchaseRequest, PurchaseRequestState, StoreDurabilityError,
};
use crate::content::store_cosmetics;
use oteryn_protocol_oteryn::store::{self as wire, cosmetic_tokens};
use sqlx::{Postgres, Transaction};

fn refused(code: wire::StorePurchaseCode) -> wire::StorePurchaseResult {
    wire::StorePurchaseResult {
        code,
        order_id: None,
        operation_id: None,
    }
}
async fn current(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterGameplayFence,
    account: Option<[u8; 16]>,
) -> Result<(), StoreDurabilityError> {
    lock_admission_relations(tx).await?;
    if assert_gameplay_fence(tx, fence, node).await?.is_err() {
        return Err(StoreDurabilityError::AuthorityRejected);
    }
    if let Some(account) = account {
        let matches: bool = sqlx::query_scalar("SELECT account_id=encode($2,'hex')::uuid FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).bind(account.as_slice()).fetch_one(&mut **tx).await?;
        if !matches {
            return Err(StoreDurabilityError::AuthorityRejected);
        }
    }
    Ok(())
}
async fn funds(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterGameplayFence,
) -> Result<(u64, i64, Option<sqlx::types::Uuid>), StoreDurabilityError> {
    let earned: Option<i64> = sqlx::query_scalar("SELECT balance FROM game_character_cosmetic_tokens WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let spent: Option<(i64, sqlx::types::Uuid)> = sqlx::query_as("SELECT spent,last_purchase_id FROM game_character_cosmetic_token_spends WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let (spent, last) = spent.map_or((0, None), |(amount, last)| (amount, Some(last)));
    let available = earned
        .unwrap_or(0)
        .checked_sub(spent)
        .and_then(|n| u64::try_from(n).ok())
        .ok_or(StoreDurabilityError::AuthorityRejected)?;
    Ok((available, spent, last))
}
async fn reserve(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterGameplayFence,
    request: &PurchaseRequest,
    offer: &str,
) -> Result<bool, StoreDurabilityError> {
    sqlx::query("INSERT INTO game_character_store_cosmetics(character_id,world_id,offer_key,purchase_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid) ON CONFLICT(character_id,world_id,offer_key) DO NOTHING")
        .bind(request.character_id.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(offer).bind(request.purchase_request_id.as_slice()).execute(&mut **tx).await?;
    let id: sqlx::types::Uuid = sqlx::query_scalar("SELECT purchase_id FROM game_character_store_cosmetics WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND offer_key=$3 FOR UPDATE")
        .bind(request.character_id.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(offer).fetch_one(&mut **tx).await?;
    Ok(id.as_bytes() == &request.purchase_request_id)
}
impl DurabilityRoot {
    pub async fn read_cosmetic_token_balance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<u64, StoreDurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let outcome = async {
                        current(&mut tx, &node, &fence, None).await?;
                        Ok(funds(&mut tx, &fence).await?.0)
                    }
                    .await;
                    if outcome.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(outcome)
                })
            })
            .await?
    }
    pub async fn purchase_store_cosmetic_tokens(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: PurchaseRequest,
        offer_key: String,
        revision: String,
    ) -> Result<wire::StorePurchaseResult, StoreDurabilityError> {
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
            let outcome = async {
                current(&mut tx, &node, &fence, Some(request.account_id)).await?;
                let offer = store_cosmetics::offer(&offer_key).ok_or(StoreDurabilityError::AuthorityRejected)?;
                match super::purchase::record_request(&mut tx, &request).await? {
                    PurchaseRequestState::Answered(bytes) => return wire::decode_purchase_result(&bytes).map_err(|_| StoreDurabilityError::ConflictingBinding),
                    PurchaseRequestState::Sent => {}
                }
                let (available, spent, last) = funds(&mut tx, &fence).await?;
                if available < offer.price { return Ok(refused(wire::StorePurchaseCode::InsufficientBalance)); }
                if !reserve(&mut tx, &fence, &request, &offer_key).await? { return Ok(refused(wire::StorePurchaseCode::OfferUnavailable)); }
                let price = i64::try_from(offer.price).map_err(|_| StoreDurabilityError::AuthorityRejected)?;
                let after = spent.checked_add(price).ok_or(StoreDurabilityError::AuthorityRejected)?;
                sqlx::query("INSERT INTO game_cosmetic_token_purchases(purchase_id,character_id,world_id,previous_purchase_id,price,spent_before,spent_after) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7)")
                    .bind(request.purchase_request_id.as_slice()).bind(request.character_id.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(last).bind(price).bind(spent).bind(after).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_cosmetic_token_spends(character_id,world_id,spent,last_purchase_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid) ON CONFLICT(character_id,world_id) DO UPDATE SET spent=EXCLUDED.spent,last_purchase_id=EXCLUDED.last_purchase_id")
                    .bind(request.character_id.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(after).bind(request.purchase_request_id.as_slice()).execute(&mut *tx).await?;
                sqlx::query("UPDATE game_character_store_cosmetics SET operation_id=purchase_id WHERE purchase_id=encode($1,'hex')::uuid AND operation_id IS NULL").bind(request.purchase_request_id.as_slice()).execute(&mut *tx).await?;
                let result = wire::StorePurchaseResult { code: wire::StorePurchaseCode::Accepted, order_id: Some(request.purchase_request_id), operation_id: Some(request.purchase_request_id) };
                let bytes = wire::encode_purchase_result(&result).map_err(|_| StoreDurabilityError::ConflictingBinding)?;
                sqlx::query("UPDATE store_purchase_requests SET state=2,answer=$2 WHERE purchase_request_id=encode($1,'hex')::uuid AND state=1").bind(request.purchase_request_id.as_slice()).bind(bytes).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_store_purchase_history(purchase_request_id,account_id,character_id,order_id,offer_key,catalog_revision,price,currency,purchased_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($1,'hex')::uuid,$4,$5,$6,$7,floor(extract(epoch FROM statement_timestamp())*1000)::bigint)")
                    .bind(request.purchase_request_id.as_slice()).bind(request.account_id.as_slice()).bind(request.character_id.as_slice()).bind(offer_key).bind(revision).bind(price).bind(cosmetic_tokens::CURRENCY).execute(&mut *tx).await?;
                Ok::<_,StoreDurabilityError>(result)
            }.await;
            if outcome.as_ref().is_ok_and(|r| r.code == wire::StorePurchaseCode::Accepted) { commit_semantic_transaction(tx, deadline).await?; }
            Ok(outcome)
        })).await?
    }
    pub async fn reserve_store_cosmetic(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: PurchaseRequest,
        offer: String,
    ) -> Result<bool, StoreDurabilityError> {
        request.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let outcome = async {
                        current(&mut tx, &node, &fence, Some(request.account_id)).await?;
                        if request.character_id != *fence.character_id.as_bytes()
                            || store_cosmetics::offer(&offer).is_none()
                        {
                            return Err(StoreDurabilityError::AuthorityRejected);
                        }
                        super::purchase::record_request(&mut tx, &request).await?;
                        reserve(&mut tx, &fence, &request, &offer).await
                    }
                    .await;
                    if outcome.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(outcome)
                })
            })
            .await?
    }
    pub async fn release_store_cosmetic(
        &self,
        request: PurchaseRequest,
    ) -> Result<(), StoreDurabilityError> {
        request.validate()?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row: Option<Vec<u8>> = sqlx::query_scalar("SELECT answer FROM store_purchase_requests WHERE purchase_request_id=encode($1,'hex')::uuid AND account_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid AND binding_digest=$4 AND state=2")
                .bind(request.purchase_request_id.as_slice()).bind(request.account_id.as_slice()).bind(request.character_id.as_slice()).bind(request.binding_digest().as_slice()).fetch_optional(&mut *tx).await?;
            let refused = row.and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()).is_some_and(|v| matches!(v["result"].as_str(), Some("STALE_QUOTE"|"OFFER_UNAVAILABLE"|"PURCHASE_DISABLED"|"INSUFFICIENT_BALANCE"|"ACCOUNT_RESTRICTED")));
            if !refused { return Ok(Err(StoreDurabilityError::ConflictingBinding)); }
            sqlx::query("DELETE FROM game_character_store_cosmetics WHERE purchase_id=encode($1,'hex')::uuid AND operation_id IS NULL").bind(request.purchase_request_id.as_slice()).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }
    pub async fn grant_store_cosmetic(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        operation: [u8; 16],
        identity: DeliveryIdentity,
        offer: String,
    ) -> Result<DeliveryReceipt, StoreDurabilityError> {
        identity.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let outcome = async {
                current(&mut tx, &node, &fence, Some(identity.account_id)).await?;
                if identity.character_id != *fence.character_id.as_bytes() || identity.world_id != *fence.runtime_scope.world_id().as_bytes() || store_cosmetics::offer(&offer).is_none() { return Err(StoreDurabilityError::AuthorityRejected); }
                let row: Option<(sqlx::types::Uuid,Option<sqlx::types::Uuid>)> = sqlx::query_as("SELECT purchase_id,operation_id FROM game_character_store_cosmetics WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND offer_key=$3 FOR UPDATE")
                    .bind(identity.character_id.as_slice()).bind(identity.world_id.as_slice()).bind(&offer).fetch_optional(&mut *tx).await?;
                let Some((purchase, previous)) = row else { return Err(StoreDurabilityError::ConflictingOperation); };
                if let Some(previous) = previous {
                    if previous.as_bytes() != &operation { return Err(StoreDurabilityError::ConflictingOperation); }
                    let bytes: Vec<u8> = sqlx::query_scalar("SELECT receipt FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid AND fulfilment_unit_id=$2").bind(operation.as_slice()).bind(&identity.fulfilment_unit_id).fetch_one(&mut *tx).await?;
                    return serde_json::from_slice(&bytes).map_err(|_| StoreDurabilityError::ConflictingOperation);
                }
                let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint").fetch_one(&mut *tx).await?;
                let receipt = DeliveryReceipt { operation_id: super::codec::uuid_string(&operation), account_id: super::codec::uuid_string(&identity.account_id), character_id: super::codec::uuid_string(&identity.character_id), world_id: super::codec::uuid_string(&identity.world_id), fulfilment_unit_id: identity.fulfilment_unit_id, result: "COMMITTED".into(), recovery_generation: recovery.recovery_generation.to_string(), decided_at: now.to_string() };
                sqlx::query("UPDATE game_character_store_cosmetics SET operation_id=encode($2,'hex')::uuid WHERE purchase_id=$1").bind(purchase).bind(operation.as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO store_delivery_receipts(operation_id,account_id,character_id,world_id,fulfilment_unit_id,result,recovery_generation,decided_at,receipt,item_instance_id,entitlement_key,purchase_request_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,'COMMITTED',$6::text::numeric,$7,$8,NULL,$9,$10)")
                    .bind(operation.as_slice()).bind(identity.account_id.as_slice()).bind(identity.character_id.as_slice()).bind(identity.world_id.as_slice()).bind(&receipt.fulfilment_unit_id).bind(recovery.recovery_generation.to_string()).bind(now).bind(super::codec::encode_receipt(&receipt)?).bind(format!("store-cosmetic:{offer}")).bind(purchase).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO store_receipt_outbox(operation_id) VALUES(encode($1,'hex')::uuid)").bind(operation.as_slice()).execute(&mut *tx).await?;
                Ok::<_,StoreDurabilityError>(receipt)
            }.await;
            if outcome.is_ok() { commit_semantic_transaction(tx, deadline).await?; }
            Ok(outcome)
        })).await?
    }
}
