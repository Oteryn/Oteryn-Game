//! Character-bound Store cosmetics and atomic earned-token spending.
use crate::durability::db::commit_semantic_read;
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
use crate::content::{native_gameplay::NativeCosmeticsView, store_cosmetics};
use crate::domain::appearance::EarnedCosmetic;
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
    sqlx::query("INSERT INTO game_character_store_cosmetics(character_id,world_id,offer_key,purchase_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid) ON CONFLICT(purchase_id) DO NOTHING")
        .bind(request.character_id.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(offer).bind(request.purchase_request_id.as_slice()).execute(&mut **tx).await?;
    let id: sqlx::types::Uuid = sqlx::query_scalar("SELECT purchase_id FROM game_character_store_cosmetics WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND offer_key=$3 AND purchase_id=encode($4,'hex')::uuid FOR UPDATE")
        .bind(request.character_id.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(offer)
        .bind(request.purchase_request_id.as_slice()).fetch_one(&mut **tx).await?;
    Ok(id.as_bytes() == &request.purchase_request_id)
}
async fn lock_account_offer(
    tx: &mut Transaction<'_, Postgres>,
    account: &[u8; 16],
    offer: &str,
) -> Result<(), StoreDurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-cosmetic:'||encode($1,'hex')||':'||$2,0))")
        .bind(account.as_slice()).bind(offer).execute(&mut **tx).await?;
    Ok(())
}

async fn already_owned(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterGameplayFence,
    content: &NativeCosmeticsView,
    offer: &store_cosmetics::StoreCosmeticOffer,
) -> Result<bool, StoreDurabilityError> {
    let target = match offer.preview.kind {
        2 => EarnedCosmetic::Outfit(&offer.unlock_key),
        3 => EarnedCosmetic::Mount(&offer.unlock_key),
        _ => return Err(StoreDurabilityError::AuthorityRejected),
    };
    Ok(super::super::character_appearance::cosmetic_owned_for_sex(
        tx, fence.character_id, content, target, None,
    ).await?)
}

/// Clear only legacy reservations with a final, recorded refusal. A Sent
/// journal must keep its reservation until its original Platform id is resolved.
async fn clear_legacy_refusals(
    tx: &mut Transaction<'_, Postgres>,
    request: &PurchaseRequest,
    offer: &str,
) -> Result<(), StoreDurabilityError> {
    sqlx::query(
        "DELETE FROM game_character_store_cosmetics c USING store_purchase_requests p
         WHERE c.purchase_id=p.purchase_request_id AND c.offer_key=$2 AND c.operation_id IS NULL
           AND p.account_id=encode($1,'hex')::uuid AND c.character_id=encode($3,'hex')::uuid
           AND p.state=2
           AND (convert_from(p.answer,'UTF8')::jsonb ->> 'result') IN
               ('STALE_QUOTE','OFFER_UNAVAILABLE','PURCHASE_DISABLED','INSUFFICIENT_BALANCE','ACCOUNT_RESTRICTED')",
    ).bind(request.account_id.as_slice()).bind(offer).bind(request.character_id.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

async fn pending_character_purchase(
    tx: &mut Transaction<'_, Postgres>,
    request: &PurchaseRequest,
    offer: &str,
) -> Result<bool, StoreDurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_store_cosmetics c
         JOIN store_purchase_requests p ON p.purchase_request_id=c.purchase_id
         WHERE c.offer_key=$2 AND c.operation_id IS NULL AND p.account_id=encode($1,'hex')::uuid
           AND c.character_id=encode($3,'hex')::uuid)",
    ).bind(request.account_id.as_slice()).bind(offer).bind(request.character_id.as_slice()).fetch_one(&mut **tx).await?)
}

pub struct PendingStorePurchase {
    pub purchase_id: [u8; 16],
    pub character_id: [u8; 16],
    pub world_id: [u8; 16],
    pub binding_digest: Vec<u8>,
}

pub struct PaidStoreCosmeticIntent {
    pub operation: [u8; 16],
    pub account: [u8; 16],
    pub order: [u8; 16],
    pub fulfilment: String,
    pub offer: String,
    pub catalogue: String,
}

impl DurabilityRoot {
    /// Find the buyer's unfinished coin purchase for a foreground confirmation.
    pub async fn pending_store_cosmetic_purchase(
        &self,
        account: [u8; 16],
        character: [u8; 16],
        offer: String,
    ) -> Result<Option<PendingStorePurchase>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row: Option<(sqlx::types::Uuid, sqlx::types::Uuid, sqlx::types::Uuid, Vec<u8>)> = sqlx::query_as(
                "SELECT c.purchase_id,c.character_id,c.world_id,p.binding_digest
                 FROM game_character_store_cosmetics c
                 JOIN store_purchase_requests p ON p.purchase_request_id=c.purchase_id
                 WHERE c.offer_key=$2 AND c.operation_id IS NULL AND p.account_id=encode($1,'hex')::uuid
                   AND c.character_id=encode($3,'hex')::uuid
                   AND (p.state=1 OR EXISTS(SELECT 1 FROM game_store_purchase_history h
                                          WHERE h.purchase_request_id=c.purchase_id))
                 ORDER BY p.created_at,c.purchase_id LIMIT 1",
            ).bind(account.as_slice()).bind(offer).bind(character.as_slice()).fetch_optional(&mut *tx).await?;
            commit_semantic_read(tx, deadline).await?;
            Ok(Ok(row.map(|(purchase_id, character_id, world_id, binding_digest)| PendingStorePurchase {
                purchase_id: *purchase_id.as_bytes(),
                character_id: *character_id.as_bytes(),
                world_id: *world_id.as_bytes(),
                binding_digest,
            })))
        })).await?
    }

    /// Page through unanswered confirmations for an order whose Platform intent
    /// names the account and offer but has no Character or purchase request ID.
    pub async fn unanswered_store_cosmetic_purchases(
        &self,
        account: [u8; 16],
        offer: String,
        after: [u8; 16],
    ) -> Result<Vec<PendingStorePurchase>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let rows: Vec<(sqlx::types::Uuid, sqlx::types::Uuid, sqlx::types::Uuid, Vec<u8>)> = sqlx::query_as(
                "SELECT c.purchase_id,c.character_id,c.world_id,p.binding_digest
                 FROM game_character_store_cosmetics c
                 JOIN store_purchase_requests p ON p.purchase_request_id=c.purchase_id
                 WHERE c.offer_key=$2 AND c.operation_id IS NULL AND p.account_id=encode($1,'hex')::uuid
                   AND p.state=1 AND c.purchase_id>encode($3,'hex')::uuid
                 ORDER BY c.purchase_id LIMIT 16",
            ).bind(account.as_slice()).bind(offer).bind(after.as_slice()).fetch_all(&mut *tx).await?;
            commit_semantic_read(tx, deadline).await?;
            Ok(Ok(rows.into_iter().map(|(purchase_id, character_id, world_id, binding_digest)| PendingStorePurchase {
                purchase_id: *purchase_id.as_bytes(),
                character_id: *character_id.as_bytes(),
                world_id: *world_id.as_bytes(),
                binding_digest,
            }).collect()))
        })).await?
    }

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
                        lock_admission_relations(&mut tx).await?;
                        current(&mut tx, &node, &fence, None).await?;
                        Ok(funds(&mut tx, &fence).await?.0)
                    }
                    .await;
                    if outcome.is_ok() {
                        commit_semantic_read(tx, deadline).await?;
                    }
                    Ok(outcome)
                })
            })
            .await?
    }
    #[allow(clippy::too_many_arguments, reason = "active cosmetic content independently resolves granted ownership")]
    pub async fn purchase_store_cosmetic_tokens(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: PurchaseRequest,
        offer_key: String,
        revision: String,
        content: NativeCosmeticsView,
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
                lock_admission_relations(&mut tx).await?;
                super::purchase::lock_request(&mut tx, &request).await?;
                current(&mut tx, &node, &fence, Some(request.account_id)).await?;
                let offer = store_cosmetics::offer(&offer_key).ok_or(StoreDurabilityError::AuthorityRejected)?;
                match super::purchase::record_request(&mut tx, &request).await? {
                    PurchaseRequestState::Answered(bytes) => return wire::decode_purchase_result(&bytes).map_err(|_| StoreDurabilityError::ConflictingBinding),
                    PurchaseRequestState::Sent => {}
                }
                lock_account_offer(&mut tx, &request.account_id, &offer_key).await?;
                clear_legacy_refusals(&mut tx, &request, &offer_key).await?;
                if already_owned(&mut tx, &fence, &content, offer).await?
                    || pending_character_purchase(&mut tx, &request, &offer_key).await?
                {
                    return Ok(refused(wire::StorePurchaseCode::OfferUnavailable));
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
        content: NativeCosmeticsView,
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
                        lock_admission_relations(&mut tx).await?;
                        super::purchase::lock_request(&mut tx, &request).await?;
                        current(&mut tx, &node, &fence, Some(request.account_id)).await?;
                        if request.character_id != *fence.character_id.as_bytes()
                            || store_cosmetics::offer(&offer).is_none()
                        {
                            return Err(StoreDurabilityError::AuthorityRejected);
                        }
                        super::purchase::record_request(&mut tx, &request).await?;
                        lock_account_offer(&mut tx, &request.account_id, &offer).await?;
                        clear_legacy_refusals(&mut tx, &request, &offer).await?;
                        let own_pending: bool = sqlx::query_scalar(
                            "SELECT EXISTS(SELECT 1 FROM game_character_store_cosmetics
                             WHERE purchase_id=encode($1,'hex')::uuid AND operation_id IS NULL AND offer_key=$2)",
                        ).bind(request.purchase_request_id.as_slice()).bind(&offer).fetch_one(&mut *tx).await?;
                        if own_pending {
                            // A previously sent request can already have debited.
                            return Ok(true);
                        }
                        let cosmetic = store_cosmetics::offer(&offer)
                            .ok_or(StoreDurabilityError::AuthorityRejected)?;
                        if already_owned(&mut tx, &fence, &content, cosmetic).await?
                            || pending_character_purchase(&mut tx, &request, &offer).await?
                        {
                            return Ok(false);
                        }
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
            let refused = row.as_deref().is_some_and(super::purchase::refused);
            if !refused { return Ok(Err(StoreDurabilityError::ConflictingBinding)); }
            sqlx::query("DELETE FROM game_character_store_cosmetics WHERE purchase_id=encode($1,'hex')::uuid AND operation_id IS NULL").bind(request.purchase_request_id.as_slice()).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }
    /// The worker can finish a paid account grant from its durable reservation
    /// and answer, even after the original Character's session or ownership ends.
    pub async fn recover_paid_store_cosmetic(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        intent: PaidStoreCosmeticIntent,
    ) -> Result<bool, StoreDurabilityError> {
        let recovery = authority.record_for(self).map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let original: Option<(sqlx::types::Uuid, sqlx::types::Uuid, sqlx::types::Uuid)> = sqlx::query_as(
                "SELECT c.purchase_id,c.character_id,c.world_id
                 FROM game_store_purchase_history h
                 JOIN game_character_store_cosmetics c ON c.purchase_id=h.purchase_request_id
                 WHERE h.account_id=encode($1,'hex')::uuid AND h.order_id=encode($2,'hex')::uuid
                   AND h.offer_key=$3 AND h.catalog_revision=$4 AND h.currency='TIBIA_COIN'
                   AND c.operation_id IS NULL",
            ).bind(intent.account.as_slice()).bind(intent.order.as_slice())
                .bind(&intent.offer).bind(&intent.catalogue).fetch_optional(&mut *tx).await?;
            let Some((purchase, character, world)) = original else { return Ok(Ok(false)); };
            let identity = DeliveryIdentity {
                account_id: intent.account,
                character_id: *character.as_bytes(),
                world_id: *world.as_bytes(),
                fulfilment_unit_id: intent.fulfilment,
            };
            if let Err(error) = identity.validate() {
                return Ok(Err(error.into()));
            }
            let outcome = apply_paid_cosmetic(
                &mut tx, recovery.recovery_generation, intent.operation, identity, *purchase.as_bytes(), &intent.offer,
            ).await;
            match outcome {
                Ok(_) => {
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(true))
                }
                Err(error) => Ok(Err(error)),
            }
        })).await?
    }

    #[allow(clippy::too_many_arguments, reason = "exact purchase id distinguishes immutable grants across transfers")]
    pub async fn grant_store_cosmetic(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        operation: [u8; 16],
        identity: DeliveryIdentity,
        purchase_request_id: [u8; 16],
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
                lock_admission_relations(&mut tx).await?;
                current(&mut tx, &node, &fence, Some(identity.account_id)).await?;
                apply_paid_cosmetic(
                    &mut tx, recovery.recovery_generation, operation, identity, purchase_request_id, &offer,
                ).await
            }.await;
            if outcome.is_ok() { commit_semantic_transaction(tx, deadline).await?; }
            Ok(outcome)
        })).await?
    }
}


async fn apply_paid_cosmetic(
    tx: &mut Transaction<'_, Postgres>,
    generation: u64,
    operation: [u8; 16],
    identity: DeliveryIdentity,
    purchase_request_id: [u8; 16],
    offer: &str,
) -> Result<DeliveryReceipt, StoreDurabilityError> {
    if store_cosmetics::offer(offer).is_none() { return Err(StoreDurabilityError::AuthorityRejected); }
    // Take the buyer root before the reservation row, including offline recovery.
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR KEY SHARE")
        .bind(identity.character_id.as_slice()).fetch_one(&mut **tx).await?;
    let row: Option<(sqlx::types::Uuid,Option<sqlx::types::Uuid>)> = sqlx::query_as(
        "SELECT c.purchase_id,c.operation_id FROM game_character_store_cosmetics c
         JOIN store_purchase_requests p ON p.purchase_request_id=c.purchase_id
         WHERE c.character_id=encode($1,'hex')::uuid AND c.world_id=encode($2,'hex')::uuid
           AND c.offer_key=$3 AND c.purchase_id=encode($4,'hex')::uuid
           AND p.account_id=encode($5,'hex')::uuid AND p.character_id=c.character_id
           AND p.state=2
           AND (convert_from(p.answer,'UTF8')::jsonb -> 'operation_ids') ? $6
         FOR UPDATE OF c",
    ).bind(identity.character_id.as_slice()).bind(identity.world_id.as_slice()).bind(offer)
        .bind(purchase_request_id.as_slice()).bind(identity.account_id.as_slice())
        .bind(super::codec::uuid_string(&operation)).fetch_optional(&mut **tx).await?;
    let Some((purchase, previous)) = row else { return Err(StoreDurabilityError::ConflictingOperation); };
    if let Some(previous) = previous {
        if previous.as_bytes() != &operation { return Err(StoreDurabilityError::ConflictingOperation); }
        let bytes: Vec<u8> = sqlx::query_scalar("SELECT receipt FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid AND fulfilment_unit_id=$2").bind(operation.as_slice()).bind(&identity.fulfilment_unit_id).fetch_one(&mut **tx).await?;
        return serde_json::from_slice(&bytes).map_err(|_| StoreDurabilityError::ConflictingOperation);
    }
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint").fetch_one(&mut **tx).await?;
    let receipt = DeliveryReceipt { operation_id: super::codec::uuid_string(&operation), account_id: super::codec::uuid_string(&identity.account_id), character_id: super::codec::uuid_string(&identity.character_id), world_id: super::codec::uuid_string(&identity.world_id), fulfilment_unit_id: identity.fulfilment_unit_id, result: "COMMITTED".into(), recovery_generation: generation.to_string(), decided_at: now.to_string() };
    sqlx::query("UPDATE game_character_store_cosmetics SET operation_id=encode($2,'hex')::uuid WHERE purchase_id=$1").bind(purchase).bind(operation.as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO store_delivery_receipts(operation_id,account_id,character_id,world_id,fulfilment_unit_id,result,recovery_generation,decided_at,receipt,item_instance_id,entitlement_key,purchase_request_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,'COMMITTED',$6::text::numeric,$7,$8,NULL,$9,$10)")
        .bind(operation.as_slice()).bind(identity.account_id.as_slice()).bind(identity.character_id.as_slice()).bind(identity.world_id.as_slice()).bind(&receipt.fulfilment_unit_id).bind(generation.to_string()).bind(now).bind(super::codec::encode_receipt(&receipt)?).bind(format!("store-cosmetic:{offer}")).bind(purchase).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO store_receipt_outbox(operation_id) VALUES(encode($1,'hex')::uuid)").bind(operation.as_slice()).execute(&mut **tx).await?;
    Ok::<_,StoreDurabilityError>(receipt)
}
