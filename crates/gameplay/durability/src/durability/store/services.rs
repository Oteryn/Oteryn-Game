//! Reserve before debit, then settle the Game effect and outbox in one transaction.
use super::super::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::CurrentCharacterGameplayFence,
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    runtime_scope_assignment::NodeIncarnationProof,
};
use super::codec::{uuid_string, uuid_v7};
use super::storage::{account_matches, assert_store_fence, lookup_delivery, write_service_receipt};
use super::{
    DeliveryIdentity, DeliveryReceipt, PurchaseRequest, PurchaseRequestState, StoreDurabilityError,
};
use sqlx::{Postgres, Row, Transaction};

const STORE_CAP: i16 = 50;

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(try_from = "i16")]
pub enum StorePreyPack {
    Five,
    Twenty,
}

impl TryFrom<i16> for StorePreyPack {
    type Error = &'static str;

    fn try_from(count: i16) -> Result<Self, Self::Error> {
        match count {
            5 => Ok(Self::Five),
            20 => Ok(Self::Twenty),
            _ => Err("unknown Crystal Prey wildcard pack"),
        }
    }
}

impl StorePreyPack {
    const fn quantity(self) -> i16 {
        match self {
            Self::Five => 5,
            Self::Twenty => 20,
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StoreService {
    PreyWildcards { count: StorePreyPack },
    XpBoost { tier: i16 },
    WeeklyTaskExpansion,
    InstantRewardAccess,
    PremiumTime { days: u16 },
    Blessings { blessing_id: Option<u8>, count: i16 },
}

impl StoreService {
    pub fn matches_price(self, price: u64) -> bool {
        match self {
            Self::PreyWildcards { .. } | Self::WeeklyTaskExpansion | Self::InstantRewardAccess => {
                price > 0
            }
            Self::PremiumTime { days } => matches!(days, 30 | 90 | 180 | 360) && price > 0,
            Self::Blessings { blessing_id, count } => {
                price > 0
                    && matches!(count, 1 | 5)
                    && super::service_blessings::keys(blessing_id).is_some()
            }
            // Crystal gamestore/init.lua ExpBoostValues. Each price is a distinct
            // fixed-price offer on Platform, selected by the current World reset.
            Self::XpBoost { tier } => match tier {
                1 => price == 30,
                2 => price == 45,
                3 => price == 90,
                4 => price == 180,
                5 => price == 360,
                _ => false,
            },
        }
    }

    const fn kind(self) -> &'static str {
        match self {
            Self::PreyWildcards { .. } => "PREY_WILDCARDS",
            Self::XpBoost { .. } => "XP_BOOST",
            Self::Blessings { .. } => "BLESSINGS",
            Self::WeeklyTaskExpansion => "WEEKLY_TASK_EXPANSION",
            Self::InstantRewardAccess => "INSTANT_REWARD_ACCESS",
            Self::PremiumTime { .. } => "PREMIUM_TIME",
        }
    }

    fn amount(self) -> i16 {
        match self {
            Self::PreyWildcards { count } => count.quantity(),
            Self::XpBoost { tier } => tier,
            Self::Blessings { count, .. } => count,
            Self::WeeklyTaskExpansion | Self::InstantRewardAccess => 1,
            Self::PremiumTime { days } => i16::try_from(days).unwrap_or_default(),
        }
    }
    pub const fn platform_kind(self) -> &'static str {
        match self {
            Self::PremiumTime { .. } => "PREMIUM_TIME",
            _ => "ITEM",
        }
    }

    pub const fn platform_quantity(self) -> u16 {
        match self {
            Self::PremiumTime { days } => days,
            _ => 1,
        }
    }

    fn blessing_keys(self) -> Result<&'static [&'static str], sqlx::Error> {
        match self {
            Self::Blessings {
                blessing_id,
                count: 1 | 5,
            } => super::service_blessings::keys(blessing_id).ok_or(sqlx::Error::Protocol(
                "invalid Store blessing identity".into(),
            )),
            Self::Blessings { .. } => {
                Err(sqlx::Error::Protocol("invalid Store blessing count".into()))
            }
            _ => Ok(&[]),
        }
    }
}

#[derive(Clone)]
pub struct StoreServiceDelivery {
    pub identity: DeliveryIdentity,
    pub request: PurchaseRequest,
    pub operation_id: [u8; 16],
    pub service: StoreService,
}

impl DurabilityRoot {
    /// Only a pre-existing reservation can restore an older service quote.
    /// The caller's price and offer fields are proven by the original binding digest.
    pub async fn read_retained_store_service(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: PurchaseRequest,
    ) -> Result<Option<StoreService>, StoreDurabilityError> {
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
                    lock_admission_relations(&mut tx).await?;
                    if &request.character_id != fence.character_id.as_bytes()
                        || !assert_store_fence(&mut tx, &fence, &node, None).await?
                        || !account_matches(&mut tx, &fence, &request.account_id).await?
                    {
                        return Ok(Err(StoreDurabilityError::AuthorityRejected));
                    }
                    let row: Option<(String, i16, Vec<String>)> = sqlx::query_as(
                        "SELECT p.kind,p.amount,p.blessing_keys FROM game_store_service_purchases p
                 JOIN store_purchase_requests r ON r.purchase_request_id=p.purchase_id
                 WHERE p.purchase_id=encode($1,'hex')::uuid AND p.account_id=encode($2,'hex')::uuid
                   AND p.character_id=encode($3,'hex')::uuid AND p.world_id=encode($4,'hex')::uuid
                   AND r.binding_digest=$5",
                    )
                    .bind(request.purchase_request_id.as_slice())
                    .bind(request.account_id.as_slice())
                    .bind(request.character_id.as_slice())
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                    .bind(request.binding_digest().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let service = match row {
                        None => None,
                        Some((kind, amount, keys)) => {
                            let Some(service) = retained_service(&kind, amount, &keys) else {
                                return Ok(Err(StoreDurabilityError::ConflictingBinding));
                            };
                            Some(service)
                        }
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(service))
                })
            })
            .await?
    }

    pub async fn read_store_xp_tier(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<i16>, StoreDurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !assert_store_fence(&mut tx, &fence, &node, None).await? {
                        return Ok(Err(StoreDurabilityError::AuthorityRejected));
                    }
                    let tier = super::super::store_hunting_xp::available_tier(
                        &mut tx,
                        fence.character_id.as_bytes(),
                        fence.runtime_scope.world_id().as_bytes(),
                    )
                    .await?
                    .map(|(_, tier)| tier);
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(tier))
                })
            })
            .await?
    }

    pub async fn reserve_store_service(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: PurchaseRequest,
        service: StoreService,
    ) -> Result<Option<PurchaseRequestState>, StoreDurabilityError> {
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
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = lock_scope(&mut tx, &fence, &node, &request).await {
                        return Ok(Err(error));
                    }
                    let state = match super::purchase::record_request(&mut tx, &request).await {
                        Ok(state) => state,
                        Err(error) => return Ok(Err(error)),
                    };
                    if state == PurchaseRequestState::Sent
                        && !reserve(
                            &mut tx,
                            &request,
                            service,
                            fence.runtime_scope.world_id().as_bytes(),
                        )
                        .await?
                    {
                        return Ok(Ok(None));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(state)))
                })
            })
            .await?
    }

    pub async fn release_refused_store_service(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        request: PurchaseRequest,
    ) -> Result<(), StoreDurabilityError> {
        request.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_account(&mut tx, &request.account_id).await?;
                    let answer: Option<Vec<u8>> = sqlx::query_scalar(
                        "SELECT answer FROM store_purchase_requests
                 WHERE purchase_request_id=encode($1,'hex')::uuid AND binding_digest=$2 AND state=2
                   AND account_id=encode($3,'hex')::uuid AND character_id=encode($4,'hex')::uuid",
                    )
                    .bind(request.purchase_request_id.as_slice())
                    .bind(request.binding_digest().as_slice())
                    .bind(request.account_id.as_slice())
                    .bind(request.character_id.as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    if !answer
                        .as_deref()
                        .is_some_and(super::prey_purchase::definitive_refusal)
                    {
                        return Ok(Err(StoreDurabilityError::ConflictingBinding));
                    }
                    sqlx::query(
                        "DELETE FROM game_store_service_purchases
                 WHERE purchase_id=encode($1,'hex')::uuid AND operation_id IS NULL",
                    )
                    .bind(request.purchase_request_id.as_slice())
                    .execute(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }

    pub async fn apply_store_service(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        delivery: StoreServiceDelivery,
    ) -> Result<DeliveryReceipt, StoreDurabilityError> {
        delivery.validate(&fence)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = lock_scope(&mut tx, &fence, &node, &delivery.request).await
                    {
                        return Ok(Err(error));
                    }
                    let receipt = match settle_service(&mut tx, &recovery, &delivery, false).await?
                    {
                        Ok(receipt) => receipt,
                        Err(error) => return Ok(Err(error)),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(receipt))
                })
            })
            .await?
    }
}

impl StoreServiceDelivery {
    pub(in crate::durability) fn validate(
        &self,
        fence: &CurrentCharacterGameplayFence,
    ) -> Result<(), StoreDurabilityError> {
        self.identity.validate()?;
        self.request.validate()?;
        if !uuid_v7(&self.operation_id)
            || self.identity.character_id != self.request.character_id
            || &self.identity.character_id != fence.character_id.as_bytes()
            || self.identity.account_id != self.request.account_id
            || &self.identity.world_id != fence.runtime_scope.world_id().as_bytes()
        {
            return Err(StoreDurabilityError::ConflictingOperation);
        }
        Ok(())
    }

    pub(in crate::durability) fn task_board_reference(&self) -> String {
        format!("store-service:{}", uuid_string(&self.operation_id))
    }
}

pub(in crate::durability) async fn settle_service(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    delivery: &StoreServiceDelivery,
    task_board_committed: bool,
) -> Result<Result<DeliveryReceipt, StoreDurabilityError>, super::super::DurabilityError> {
    let StoreServiceDelivery {
        identity,
        request,
        operation_id,
        service,
    } = delivery;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-delivery:'||encode($1,'hex'),0))")
        .bind(operation_id.as_slice()).execute(&mut **tx).await?;
    let state = match settlement_state(tx, request, identity, *operation_id, *service).await? {
        Some(state) => state,
        None => return Ok(Err(StoreDurabilityError::ConflictingBinding)),
    };
    match lookup_delivery(tx, *operation_id, identity, None).await? {
        Ok(Some(receipt)) if state => return Ok(Ok(receipt)),
        Ok(None) if !state => {}
        _ => return Ok(Err(StoreDurabilityError::ConflictingOperation)),
    }
    if matches!(service, StoreService::WeeklyTaskExpansion) && !task_board_committed {
        return Ok(Err(StoreDurabilityError::ConflictingOperation));
    }
    let receipt = match write_service_receipt(tx, recovery, *operation_id, identity).await {
        Ok(receipt) => receipt,
        Err(error) => return Ok(Err(error)),
    };
    sqlx::query(
        "UPDATE game_store_service_purchases SET operation_id=encode($2,'hex')::uuid
                 WHERE purchase_id=encode($1,'hex')::uuid",
    )
    .bind(request.purchase_request_id.as_slice())
    .bind(operation_id.as_slice())
    .execute(&mut **tx)
    .await?;
    match service {
        StoreService::PreyWildcards { count } => {
            credit(tx, identity, *operation_id, count.quantity()).await?
        }
        StoreService::XpBoost { .. } => {
            super::super::store_hunting_xp::grant(tx, identity, *operation_id).await?
        }
        StoreService::Blessings { count, .. } => {
            super::service_blessings::grant(
                tx,
                identity,
                *operation_id,
                service.blessing_keys()?,
                *count,
            )
            .await?
        }
        // The Platform purchase transaction has already granted the days. Its
        // authenticated intent registers delivery. Game's premium snapshot remains authoritative here.
        StoreService::WeeklyTaskExpansion
        | StoreService::InstantRewardAccess
        | StoreService::PremiumTime { .. } => {}
    }
    Ok(Ok(receipt))
}

fn retained_service(kind: &str, amount: i16, keys: &[String]) -> Option<StoreService> {
    match kind {
        "PREY_WILDCARDS" if keys.is_empty() => Some(StoreService::PreyWildcards {
            count: amount.try_into().ok()?,
        }),
        "XP_BOOST" if keys.is_empty() && (1..=5).contains(&amount) => {
            Some(StoreService::XpBoost { tier: amount })
        }
        "BLESSINGS" if matches!(amount, 1 | 5) => {
            let blessing_id = if super::service_blessings::keys(None).is_some_and(|all| keys == all)
            {
                None
            } else {
                Some((1..=8).find(|id| {
                    super::service_blessings::keys(Some(*id)).is_some_and(|known| keys == known)
                })?)
            };
            Some(StoreService::Blessings {
                blessing_id,
                count: amount,
            })
        }
        "WEEKLY_TASK_EXPANSION" if keys.is_empty() && amount == 1 => {
            Some(StoreService::WeeklyTaskExpansion)
        }
        "INSTANT_REWARD_ACCESS" if keys.is_empty() && amount == 1 => {
            Some(StoreService::InstantRewardAccess)
        }
        "PREMIUM_TIME" if keys.is_empty() && matches!(amount, 30 | 90 | 180 | 360) => {
            Some(StoreService::PremiumTime {
                days: u16::try_from(amount).ok()?,
            })
        }
        _ => None,
    }
}

async fn lock_scope(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterGameplayFence,
    node: &NodeIncarnationProof,
    request: &PurchaseRequest,
) -> Result<(), StoreDurabilityError> {
    if &request.character_id != fence.character_id.as_bytes()
        || !assert_store_fence(tx, fence, node, None).await?
        || !account_matches(tx, fence, &request.account_id).await?
    {
        return Err(StoreDurabilityError::AuthorityRejected);
    }
    lock_account(tx, &request.account_id).await?;
    Ok(())
}

async fn lock_account(
    tx: &mut Transaction<'_, Postgres>,
    account: &[u8; 16],
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(account.as_slice()).fetch_one(&mut **tx).await?;
    Ok(())
}

async fn reserve(
    tx: &mut Transaction<'_, Postgres>,
    request: &PurchaseRequest,
    service: StoreService,
    world: &[u8; 16],
) -> Result<bool, sqlx::Error> {
    let keys = service.blessing_keys()?;
    let pending = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_store_service_purchases
         WHERE purchase_id=encode($1,'hex')::uuid AND account_id=encode($2,'hex')::uuid
           AND world_id=encode($3,'hex')::uuid AND amount=$4 AND kind=$5 AND blessing_keys=$6 AND operation_id IS NULL)",
    )
    .bind(request.purchase_request_id.as_slice())
    .bind(request.account_id.as_slice())
    .bind(world.as_slice())
    .bind(service.amount())
    .bind(service.kind())
    .bind(keys)
    .fetch_one(&mut **tx)
    .await?;
    if pending {
        return Ok(true);
    }
    let reset = match service {
        StoreService::PreyWildcards { .. } => {
            let balance: i16 = sqlx::query_scalar(
                "SELECT coalesce((SELECT balance FROM game_account_prey_wildcards
                 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid),0)::smallint",
            ).bind(request.account_id.as_slice()).bind(world.as_slice()).fetch_one(&mut **tx).await?;
            if balance + service.amount() > STORE_CAP {
                return Ok(false);
            }
            None
        }
        StoreService::XpBoost { tier } => {
            let Some((epoch, current_tier)) =
                super::super::store_hunting_xp::available_tier(tx, &request.character_id, world)
                    .await?
            else {
                return Ok(false);
            };
            if current_tier != tier {
                return Ok(false);
            }
            Some(epoch)
        }
        StoreService::Blessings { count, .. } => {
            if !super::service_blessings::can_reserve(tx, &request.character_id, keys, count)
                .await?
            {
                return Ok(false);
            }
            None
        }
        StoreService::WeeklyTaskExpansion => {
            let available: bool = sqlx::query_scalar(
                "SELECT game_task_board_period(encode($2,'hex')::uuid) IS NOT NULL
                   AND NOT EXISTS (SELECT 1 FROM game_task_board_expansion_deliveries
                                   WHERE character_id=encode($1,'hex')::uuid)",
            )
            .bind(request.character_id.as_slice())
            .bind(world.as_slice())
            .fetch_one(&mut **tx)
            .await?;
            if !available {
                return Ok(false);
            }
            None
        }
        StoreService::InstantRewardAccess => {
            if super::reward_access::available(tx, &request.character_id, world).await? >= 90 {
                return Ok(false);
            }
            None
        }
        StoreService::PremiumTime { days: 30 | 90 | 180 | 360 } => None,
        StoreService::PremiumTime { .. } => return Ok(false),
    };
    let inserted = sqlx::query(
        "INSERT INTO game_store_service_purchases(purchase_id,account_id,world_id,character_id,kind,amount,reset_epoch,blessing_keys)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7::text::numeric,$8)
         ON CONFLICT DO NOTHING",
    ).bind(request.purchase_request_id.as_slice()).bind(request.account_id.as_slice())
        .bind(world.as_slice()).bind(request.character_id.as_slice()).bind(service.kind())
        .bind(service.amount()).bind(reset).bind(keys).execute(&mut **tx).await?;
    Ok(inserted.rows_affected() == 1)
}

/// Only an accepted operation of this exact reserved request can settle or replay the grant.
async fn settlement_state(
    tx: &mut Transaction<'_, Postgres>,
    request: &PurchaseRequest,
    identity: &DeliveryIdentity,
    operation: [u8; 16],
    service: StoreService,
) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT p.operation_id IS NOT NULL FROM game_store_service_purchases p
         JOIN store_purchase_requests r ON r.purchase_request_id=p.purchase_id
         WHERE p.purchase_id=encode($1,'hex')::uuid AND p.account_id=encode($2,'hex')::uuid
           AND p.world_id=encode($3,'hex')::uuid AND p.amount=$4 AND p.kind=$8 AND p.blessing_keys=$9 AND r.character_id=encode($5,'hex')::uuid AND r.character_id=p.character_id
           AND r.account_id=p.account_id AND r.binding_digest=$6 AND r.state=2
           AND (p.operation_id IS NULL OR p.operation_id=$7::text::uuid)
           AND convert_from(r.answer,'UTF8')::jsonb->>'result'='ACCEPTED'
           AND convert_from(r.answer,'UTF8')::jsonb->'operation_ids'=jsonb_build_array($7::text)",
    ).bind(request.purchase_request_id.as_slice()).bind(identity.account_id.as_slice())
        .bind(identity.world_id.as_slice()).bind(service.amount()).bind(identity.character_id.as_slice())
        .bind(request.binding_digest().as_slice()).bind(uuid_string(&operation)).bind(service.kind()).bind(service.blessing_keys()?).fetch_optional(&mut **tx).await
}

async fn credit(
    tx: &mut Transaction<'_, Postgres>,
    identity: &DeliveryIdentity,
    operation: [u8; 16],
    quantity: i16,
) -> Result<(), sqlx::Error> {
    let row = sqlx::query(
        "SELECT balance,last_claim_id::text FROM game_account_prey_wildcards
         WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE",
    )
    .bind(identity.account_id.as_slice())
    .bind(identity.world_id.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let (before, last): (i16, Option<String>) = match row {
        Some(row) => (row.try_get("balance")?, Some(row.try_get("last_claim_id")?)),
        None => (0, None),
    };
    let after = before + quantity;
    sqlx::query(
        "INSERT INTO game_account_prey_wildcard_entries
         (claim_id,account_id,world_id,previous_claim_id,kind,requested,credited,balance_before,balance_after)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::uuid,'STORE',$5,$5,$6,$7)",
    ).bind(operation.as_slice()).bind(identity.account_id.as_slice()).bind(identity.world_id.as_slice())
        .bind(last).bind(quantity).bind(before).bind(after).execute(&mut **tx).await?;
    sqlx::query(
        "INSERT INTO game_account_prey_wildcards(account_id,world_id,balance,last_claim_id)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid)
         ON CONFLICT(account_id,world_id) DO UPDATE SET balance=EXCLUDED.balance,last_claim_id=EXCLUDED.last_claim_id",
    ).bind(identity.account_id.as_slice()).bind(identity.world_id.as_slice()).bind(after)
        .bind(operation.as_slice()).execute(&mut **tx).await?;
    Ok(())
}
