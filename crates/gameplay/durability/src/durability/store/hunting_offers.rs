//! Game-owned reservations for bounded Store effects. Platform owns the debit.
use super::super::DurabilityRoot;
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::CurrentCharacterGameplayFence;
use super::super::character_wheel::WheelPremiumGate;
use super::super::db::{
    begin_semantic_transaction, commit_semantic_read, commit_semantic_transaction,
    lock_admission_relations,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::codec::{DeliveryIdentity, DeliveryReceipt, StoreValidationError, uuid_v7};
use super::storage::{ReceiptPayload, account_matches, assert_store_fence, write_receipt};
use super::{PurchaseRequest, StoreDurabilityError};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StoreHuntingOffer {
    XpBoost(i16),
    PermanentPreySlot,
    PreyWildcards,
}

impl StoreHuntingOffer {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "store:xp-boost:1" => Some(Self::XpBoost(1)),
            "store:xp-boost:2" => Some(Self::XpBoost(2)),
            "store:xp-boost:3" => Some(Self::XpBoost(3)),
            "store:xp-boost:4" => Some(Self::XpBoost(4)),
            "store:xp-boost:5" => Some(Self::XpBoost(5)),
            "store:permanent-prey-slot" => Some(Self::PermanentPreySlot),
            "store:prey-wildcards:1" | "store:prey-wildcards:5" => Some(Self::PreyWildcards),
            _ => None,
        }
    }
}

pub struct PendingStoreHuntingPurchase {
    pub request: PurchaseRequest,
    pub world_id: [u8; 16],
    pub offer_key: String,
    pub catalog_revision: String,
    pub price: i64,
}

pub struct StoreHuntingPurchase {
    pub request: PurchaseRequest,
    pub offer_key: String,
    pub allow_new: bool,
    pub catalog_revision: String,
    pub price: i64,
}

pub struct PaidStoreHuntingIntent {
    pub operation: [u8; 16],
    pub account: [u8; 16],
    pub order: [u8; 16],
    pub fulfilment: String,
    pub offer: String,
    pub catalogue: String,
}

pub struct StoreHuntingDelivery {
    pub identity: DeliveryIdentity,
    pub operation_id: [u8; 16],
    pub purchase_request_id: [u8; 16],
    pub offer_key: String,
}

impl DurabilityRoot {
    pub async fn next_store_xp_boost_tier(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        origin_channel: [u8; 16],
    ) -> Result<Option<i16>, StoreDurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_store_fence(&mut tx, &fence, &node, Some(origin_channel)).await? {
                return Ok(Err(StoreDurabilityError::AuthorityRejected));
            }
            let tier = sqlx::query_scalar("SELECT tier FROM game_store_xp_boost_tier(encode($1,'hex')::uuid,encode($2,'hex')::uuid) WHERE tier <= 5")
                .bind(fence.character_id.as_bytes().as_slice())
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .fetch_optional(&mut *tx).await?;
            commit_semantic_read(tx, deadline).await?;
            Ok(Ok(tier))
        })).await?
    }

    pub async fn reserve_store_hunting_purchase<G: WheelPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        plan: StoreHuntingPurchase,
        gate: &G,
        origin_channel: [u8; 16],
    ) -> Result<Option<bool>, StoreDurabilityError> {
        plan.request.validate()?;
        let offer = StoreHuntingOffer::from_key(&plan.offer_key)
            .ok_or(StoreValidationError::InvalidText)?;
        if plan.request.character_id != *fence.character_id.as_bytes() {
            return Err(StoreDurabilityError::AuthorityRejected);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        let mut gate = gate;
        self.try_issue_semantic_pass()?.run_with_context(&mut gate, move |holder, deadline, gate| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let premium = offer == StoreHuntingOffer::PermanentPreySlot
                && gate.allocation_permitted(&mut tx, plan.request.account_id).await?;
            if !assert_store_fence(&mut tx, &fence, &node, Some(origin_channel)).await? {
                return Ok(Err(StoreDurabilityError::AuthorityRejected));
            }
            let reserved = sqlx::query_scalar("SELECT game_store_reserve_hunting_purchase(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8,$9,$10,$11::text::numeric)")
                .bind(plan.request.purchase_request_id.as_slice())
                .bind(plan.request.account_id.as_slice())
                .bind(plan.request.character_id.as_slice())
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(plan.request.binding_digest().as_slice())
                .bind(&plan.offer_key)
                .bind(premium)
                .bind(plan.allow_new)
                .bind(&plan.catalog_revision)
                .bind(plan.price)
                .bind(recovery.recovery_generation.to_string())
                .fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(reserved))
        })).await?
    }

    pub async fn mark_store_hunting_purchase_sent(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        purchase_id: [u8; 16],
    ) -> Result<bool, StoreDurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let sent = sqlx::query_scalar(
                        "SELECT game_store_mark_hunting_sent(encode($1,'hex')::uuid,$2::text::numeric)",
                    )
                    .bind(purchase_id.as_slice())
                    .bind(recovery.recovery_generation.to_string())
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(sent))
                })
            })
            .await?
    }

    /// Sent confirmations are safe to replay by their exact immutable Platform purchase id.
    /// An unsent hold is only reclaimed by the account lock on the next reservation.
    pub async fn pending_store_hunting_purchases(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        after: [u8; 16],
    ) -> Result<Vec<PendingStoreHuntingPurchase>, StoreDurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let rows = sqlx::query_as::<_, (sqlx::types::Uuid, sqlx::types::Uuid, sqlx::types::Uuid, sqlx::types::Uuid, Vec<u8>, String, String, i64)>(
                "SELECT r.purchase_id,r.account_id,r.character_id,r.world_id,p.binding_digest,
                        r.offer_key,r.catalog_revision,r.price
                 FROM game_store_hunting_pending r JOIN store_purchase_requests p ON p.purchase_request_id=r.purchase_id
                 WHERE r.sent AND r.operation_id IS NULL AND p.state=1
                   AND r.recovery_generation=$2::text::numeric
                   AND r.purchase_id>encode($1,'hex')::uuid ORDER BY r.purchase_id LIMIT 16",
            ).bind(after.as_slice()).bind(recovery.recovery_generation.to_string()).fetch_all(&mut *tx).await?;
            commit_semantic_read(tx, deadline).await?;
            Ok(Ok(rows.into_iter().map(|(purchase, account, character, world, binding, offer_key, catalog_revision, price)| PendingStoreHuntingPurchase {
                request: PurchaseRequest {
                    purchase_request_id: *purchase.as_bytes(),
                    account_id: *account.as_bytes(),
                    character_id: *character.as_bytes(),
                    binding,
                },
                world_id: *world.as_bytes(), offer_key, catalog_revision, price,
            }).collect()))
        })).await?
    }

    pub async fn pending_store_hunting_confirmation(
        &self,
        quote: PendingStoreHuntingPurchase,
    ) -> Result<Option<PurchaseRequest>, StoreDurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row = sqlx::query_as::<_, (sqlx::types::Uuid, Vec<u8>)>(
                "SELECT r.purchase_id,p.binding_digest FROM game_store_hunting_pending r
                 JOIN store_purchase_requests p ON p.purchase_request_id=r.purchase_id
                 WHERE r.account_id=encode($1,'hex')::uuid AND r.character_id=encode($2,'hex')::uuid
                   AND r.world_id=encode($3,'hex')::uuid AND r.offer_key=$4 AND r.catalog_revision=$5
                   AND r.price=$6 AND (r.sent OR r.reserved_at>clock_timestamp()-interval '30 seconds')
                 LIMIT 1",
            ).bind(quote.request.account_id.as_slice()).bind(quote.request.character_id.as_slice())
                .bind(quote.world_id.as_slice()).bind(&quote.offer_key).bind(&quote.catalog_revision)
                .bind(quote.price).fetch_optional(&mut *tx).await?;
            commit_semantic_read(tx, deadline).await?;
            Ok(Ok(row.map(|(purchase, binding)| PurchaseRequest {
                purchase_request_id: *purchase.as_bytes(), binding, ..quote.request
            })))
        })).await?
    }

    /// The recovery path needs the accepted Platform operation and the exact reserved order.
    /// It does not require the Character to have an online session.
    pub async fn recover_store_hunting_offer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        intent: PaidStoreHuntingIntent,
    ) -> Result<Option<DeliveryReceipt>, StoreDurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_operation(&mut tx, intent.operation).await?;
            let row = sqlx::query_as::<_, (sqlx::types::Uuid, sqlx::types::Uuid, sqlx::types::Uuid)>(
                "SELECT r.purchase_id,r.character_id,r.world_id FROM game_store_hunting_pending r
                 JOIN game_store_purchase_history h ON h.purchase_request_id=r.purchase_id
                 WHERE r.operation_id=encode($1,'hex')::uuid AND r.account_id=encode($2,'hex')::uuid
                   AND h.order_id=encode($3,'hex')::uuid AND r.offer_key=$4 AND r.catalog_revision=$5",
            ).bind(intent.operation.as_slice()).bind(intent.account.as_slice()).bind(intent.order.as_slice())
                .bind(&intent.offer).bind(&intent.catalogue).fetch_optional(&mut *tx).await?;
            let Some((purchase, character, world)) = row else { return Ok(Ok(None)); };
            let plan = StoreHuntingDelivery {
                identity: DeliveryIdentity {
                    account_id: intent.account, character_id: *character.as_bytes(),
                    world_id: *world.as_bytes(), fulfilment_unit_id: intent.fulfilment,
                },
                operation_id: intent.operation, purchase_request_id: *purchase.as_bytes(), offer_key: intent.offer,
            };
            if let Err(error) = plan.identity.validate() {
                return Ok(Err(error.into()));
            }
            match write_hunting_receipt(&mut tx, &recovery, &plan).await {
                Ok(receipt) => {
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(receipt)))
                }
                Err(error) => Ok(Err(error)),
            }
        })).await?
    }

    pub async fn grant_store_hunting_offer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        plan: StoreHuntingDelivery,
        origin_channel: [u8; 16],
    ) -> Result<DeliveryReceipt, StoreDurabilityError> {
        plan.identity.validate()?;
        if StoreHuntingOffer::from_key(&plan.offer_key).is_none() {
            return Err(StoreValidationError::InvalidText.into());
        }
        if !uuid_v7(&plan.operation_id) || !uuid_v7(&plan.purchase_request_id) {
            return Err(StoreValidationError::InvalidUuid.into());
        }
        if plan.identity.character_id != *fence.character_id.as_bytes()
            || plan.identity.world_id != *fence.runtime_scope.world_id().as_bytes()
        {
            return Err(StoreDurabilityError::AuthorityRejected);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_operation(&mut tx, plan.operation_id).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_store_fence(&mut tx, &fence, &node, Some(origin_channel)).await?
                || !account_matches(&mut tx, &fence, &plan.identity.account_id).await? {
                return Ok(Err(StoreDurabilityError::AuthorityRejected));
            }
            let existing: Option<(bool, Vec<u8>)> = sqlx::query_as(
                "SELECT ROW(account_id,character_id,world_id,fulfilment_unit_id,purchase_request_id,entitlement_key)
                     IS NOT DISTINCT FROM ROW(encode($2,'hex')::uuid,encode($3,'hex')::uuid,
                         encode($4,'hex')::uuid,$5::text,encode($6,'hex')::uuid,$7::text), receipt
                   FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid",
            ).bind(plan.operation_id.as_slice())
                .bind(plan.identity.account_id.as_slice())
                .bind(plan.identity.character_id.as_slice())
                .bind(plan.identity.world_id.as_slice())
                .bind(&plan.identity.fulfilment_unit_id)
                .bind(plan.purchase_request_id.as_slice())
                .bind(&plan.offer_key)
                .fetch_optional(&mut *tx).await?;
            let receipt = match existing {
                Some((false, _)) => return Ok(Err(StoreDurabilityError::ConflictingOperation)),
                Some((true, bytes)) => serde_json::from_slice(&bytes)
                    .map_err(|_| super::super::DurabilityError::InvalidStoredState)?,
                None => match write_hunting_receipt(&mut tx, &recovery, &plan).await {
                    Ok(receipt) => receipt,
                    Err(error) => return Ok(Err(error)),
                },
            };
            commit_hunting_grant(tx, deadline).await?;
            Ok(Ok(receipt))
        })).await?
    }
}

#[cfg(any(test, feature = "test-support"))]
tokio::task_local! {
    static STALL_HUNTING_GRANT_COMMIT: std::cell::Cell<bool>;
}

#[cfg(any(test, feature = "test-support"))]
pub async fn with_stalled_hunting_grant_commit<F: std::future::Future>(
    operation: F,
) -> (F::Output, bool) {
    STALL_HUNTING_GRANT_COMMIT
        .scope(std::cell::Cell::new(false), async {
            let result = operation.await;
            (
                result,
                STALL_HUNTING_GRANT_COMMIT.with(std::cell::Cell::get),
            )
        })
        .await
}

async fn commit_hunting_grant(
    tx: sqlx::Transaction<'_, sqlx::Postgres>,
    deadline: tokio::time::Instant,
) -> Result<(), super::super::DurabilityError> {
    #[cfg(any(test, feature = "test-support"))]
    if STALL_HUNTING_GRANT_COMMIT.try_with(|_| ()).is_ok() {
        let (result, reached) = super::super::db::with_stalled_commit_response(
            commit_semantic_transaction(tx, deadline),
        )
        .await;
        STALL_HUNTING_GRANT_COMMIT.with(|flag| flag.set(reached));
        return result;
    }
    commit_semantic_transaction(tx, deadline).await
}

async fn lock_operation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    operation: [u8; 16],
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-delivery:'||encode($1,'hex'),0))")
        .bind(operation.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

async fn write_hunting_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    plan: &StoreHuntingDelivery,
) -> Result<DeliveryReceipt, StoreDurabilityError> {
    write_receipt(
        tx,
        recovery,
        plan.operation_id,
        &plan.identity,
        ReceiptPayload {
            purchase_request_id: Some(plan.purchase_request_id),
            entitlement_key: Some(&plan.offer_key),
            ..ReceiptPayload::default()
        },
    )
    .await
}
