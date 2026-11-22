//! Store packs credit the same Account + World ledger as Daily Reward and Prey spend.
use super::super::{
    DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::CurrentCharacterGameplayFence,
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    runtime_scope_assignment::NodeIncarnationProof,
};
use super::codec::{uuid_string, uuid_v7};
use super::storage::{account_matches, assert_store_fence, lookup_delivery, write_receipt};
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

pub struct StorePreyDelivery {
    pub identity: DeliveryIdentity,
    pub request: PurchaseRequest,
    pub operation_id: [u8; 16],
    pub pack: StorePreyPack,
}

impl DurabilityRoot {
    pub async fn reserve_store_prey_pack(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: PurchaseRequest,
        pack: StorePreyPack,
    ) -> Result<Option<PurchaseRequestState>, StoreDurabilityError> {
        request.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
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
            if state == PurchaseRequestState::Sent {
                let world = fence.runtime_scope.world_id();
                let reserved = sqlx::query(
                    "INSERT INTO game_store_prey_purchases(purchase_id,account_id,world_id,quantity)
                     SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4
                     WHERE coalesce((SELECT balance FROM game_account_prey_wildcards
                        WHERE account_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid),0)+$4<=$5
                     ON CONFLICT DO NOTHING",
                ).bind(request.purchase_request_id.as_slice()).bind(request.account_id.as_slice())
                    .bind(world.as_bytes().as_slice()).bind(pack.quantity()).bind(STORE_CAP)
                    .execute(&mut *tx).await?.rows_affected() == 1;
                if !reserved && !has_pending_pack(&mut tx, &request, pack, world.as_bytes()).await? {
                    return Ok(Ok(None));
                }
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(Some(state)))
        })).await?
    }

    pub async fn release_refused_store_prey_pack(
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
                        "DELETE FROM game_store_prey_purchases
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

    pub async fn apply_store_prey_pack(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        delivery: StorePreyDelivery,
    ) -> Result<DeliveryReceipt, StoreDurabilityError> {
        let StorePreyDelivery {
            identity,
            request,
            operation_id,
            pack,
        } = delivery;
        identity.validate()?;
        request.validate()?;
        if !uuid_v7(&operation_id)
            || identity.character_id != request.character_id
            || identity.account_id != request.account_id
            || &identity.world_id != fence.runtime_scope.world_id().as_bytes()
        {
            return Err(StoreDurabilityError::ConflictingOperation);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error) = lock_scope(&mut tx, &fence, &node, &request).await {
                return Ok(Err(error));
            }
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-delivery:'||encode($1,'hex'),0))")
                .bind(operation_id.as_slice()).execute(&mut *tx).await?;
            let state = match settlement_state(&mut tx, &request, &identity, operation_id, pack).await? {
                Some(state) => state,
                None => return Ok(Err(StoreDurabilityError::ConflictingBinding)),
            };
            match lookup_delivery(&mut tx, operation_id, &identity, None).await? {
                Ok(Some(receipt)) if state => {
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(Ok(receipt));
                }
                Ok(None) if !state => {}
                _ => return Ok(Err(StoreDurabilityError::ConflictingOperation)),
            }
            let receipt = match write_receipt(&mut tx, &recovery, operation_id, &identity, None, None, None).await {
                Ok(receipt) => receipt,
                Err(error) => return Ok(Err(error)),
            };
            sqlx::query("UPDATE game_store_prey_purchases SET operation_id=encode($2,'hex')::uuid
                         WHERE purchase_id=encode($1,'hex')::uuid")
                .bind(request.purchase_request_id.as_slice()).bind(operation_id.as_slice())
                .execute(&mut *tx).await?;
            credit(&mut tx, &identity, operation_id, pack.quantity()).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(receipt))
        })).await?
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

async fn has_pending_pack(
    tx: &mut Transaction<'_, Postgres>,
    request: &PurchaseRequest,
    pack: StorePreyPack,
    world: &[u8; 16],
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_store_prey_purchases
         WHERE purchase_id=encode($1,'hex')::uuid AND account_id=encode($2,'hex')::uuid
           AND world_id=encode($3,'hex')::uuid AND quantity=$4 AND operation_id IS NULL)",
    )
    .bind(request.purchase_request_id.as_slice())
    .bind(request.account_id.as_slice())
    .bind(world.as_slice())
    .bind(pack.quantity())
    .fetch_one(&mut **tx)
    .await
}

/// Only an accepted operation of this exact reserved request can settle or replay the grant.
async fn settlement_state(
    tx: &mut Transaction<'_, Postgres>,
    request: &PurchaseRequest,
    identity: &DeliveryIdentity,
    operation: [u8; 16],
    pack: StorePreyPack,
) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT p.operation_id IS NOT NULL FROM game_store_prey_purchases p
         JOIN store_purchase_requests r ON r.purchase_request_id=p.purchase_id
         WHERE p.purchase_id=encode($1,'hex')::uuid AND p.account_id=encode($2,'hex')::uuid
           AND p.world_id=encode($3,'hex')::uuid AND p.quantity=$4 AND r.character_id=encode($5,'hex')::uuid
           AND r.account_id=p.account_id AND r.binding_digest=$6 AND r.state=2
           AND (p.operation_id IS NULL OR p.operation_id=$7::text::uuid)
           AND convert_from(r.answer,'UTF8')::jsonb->>'result'='ACCEPTED'
           AND convert_from(r.answer,'UTF8')::jsonb->'operation_ids'=jsonb_build_array($7::text)",
    ).bind(request.purchase_request_id.as_slice()).bind(identity.account_id.as_slice())
        .bind(identity.world_id.as_slice()).bind(pack.quantity()).bind(identity.character_id.as_slice())
        .bind(request.binding_digest().as_slice()).bind(uuid_string(&operation)).fetch_optional(&mut **tx).await
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
