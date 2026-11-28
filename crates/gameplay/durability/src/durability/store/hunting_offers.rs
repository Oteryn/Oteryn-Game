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

pub struct StoreHuntingPurchase {
    pub request: PurchaseRequest,
    pub offer_key: String,
    pub allow_new: bool,
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
    ) -> Result<Option<i16>, StoreDurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_store_fence(&mut tx, &fence, &node, None).await? {
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
    ) -> Result<bool, StoreDurabilityError> {
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
            if !assert_store_fence(&mut tx, &fence, &node, None).await? {
                return Ok(Err(StoreDurabilityError::AuthorityRejected));
            }
            let reserved = sqlx::query_scalar("SELECT game_store_reserve_hunting_purchase(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8)")
                .bind(plan.request.purchase_request_id.as_slice())
                .bind(plan.request.account_id.as_slice())
                .bind(plan.request.character_id.as_slice())
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(plan.request.binding_digest().as_slice())
                .bind(&plan.offer_key)
                .bind(premium)
                .bind(plan.allow_new)
                .fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(reserved))
        })).await?
    }

    pub async fn release_refused_store_hunting_purchase(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        purchase_id: [u8; 16],
    ) -> Result<(), StoreDurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    sqlx::query(
                        "SELECT game_store_release_hunting_purchase(encode($1,'hex')::uuid)",
                    )
                    .bind(purchase_id.as_slice())
                    .execute(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }

    pub async fn grant_store_hunting_offer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        plan: StoreHuntingDelivery,
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
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-delivery:'||encode($1,'hex'),0))")
                .bind(plan.operation_id.as_slice()).execute(&mut *tx).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_store_fence(&mut tx, &fence, &node, None).await?
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
                None => match write_receipt(&mut tx, &recovery, plan.operation_id, &plan.identity,
                    ReceiptPayload {
                        purchase_request_id: Some(plan.purchase_request_id),
                        entitlement_key: Some(&plan.offer_key),
                        ..ReceiptPayload::default()
                    }).await {
                        Ok(receipt) => receipt,
                        Err(error) => return Ok(Err(error)),
                    }
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(receipt))
        })).await?
    }
}
