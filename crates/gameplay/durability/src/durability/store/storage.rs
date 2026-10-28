use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence};
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use super::codec::{
    DeliveryPlan, DeliveryReceipt, StoreValidationError, encode_receipt, uuid_string, uuid_v7,
};
use sqlx::Row;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseRequestState {
    Sent,
    Answered(Vec<u8>),
}
#[derive(Debug)]
pub enum StoreDurabilityError {
    Invalid(StoreValidationError),
    ConflictingBinding,
    ConflictingOperation,
    AuthorityRejected,
    Unavailable(DurabilityError),
    Inbox(crate::durability::character_inbox::RewardClaimInboxError),
}
impl From<StoreValidationError> for StoreDurabilityError {
    fn from(v: StoreValidationError) -> Self {
        Self::Invalid(v)
    }
}
impl From<DurabilityError> for StoreDurabilityError {
    fn from(v: DurabilityError) -> Self {
        Self::Unavailable(v)
    }
}
impl From<sqlx::Error> for StoreDurabilityError {
    fn from(v: sqlx::Error) -> Self {
        Self::Unavailable(v.into())
    }
}
impl DurabilityRoot {
    /// Mint through the native Inbox owner; its audit, placement and this receipt
    /// and outbox commit together. Completed operations reconcile without reacquiring
    /// session authority. Platform remains the sole debit/delivery ledger authority.
    pub async fn apply_store_delivery(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        operation_id: [u8; 16],
        plan: DeliveryPlan,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<DeliveryReceipt, StoreDurabilityError> {
        if !uuid_v7(&operation_id) {
            return Err(StoreValidationError::InvalidUuid.into());
        }
        plan.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        if let Some(receipt) = self
            .read_store_delivery(authority, operation_id, &plan)
            .await?
        {
            return Ok(receipt);
        }
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(StoreDurabilityError::AuthorityRejected);
        };
        if plan.character_id != *fence.character_id.as_bytes()
            || plan.world_id != *world_id.as_bytes()
            || plan.command.game_session_id() != fence.game_session_id
        {
            return Err(StoreDurabilityError::AuthorityRejected);
        }
        // Refuse invalid account/world/session before reserving any mint work. The
        // native commit checks session authority again, and the callback rechecks
        // the full gameplay/recovery fence in that same value transaction.
        let account = plan.account_id;
        let proof = recovery.clone();
        let current_node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &proof).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_gameplay_fence(&mut tx, &fence, &current_node)
                        .await?
                        .is_err()
                        || !account_matches(&mut tx, &fence, &account).await?
                    {
                        return Ok(Err(StoreDurabilityError::AuthorityRejected));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await??;
        use crate::durability::character_inbox::{RewardClaimInboxError, RewardClaimInboxPlan};
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        digest.update(b"oteryn:store-delivery:v1\0");
        digest.update(operation_id);
        digest.update(plan.account_id);
        digest.update(plan.character_id);
        digest.update(plan.world_id);
        digest.update(plan.fulfilment_unit_id.as_bytes());
        let source = RewardClaimInboxPlan {
            command: plan.command,
            character_id: fence.character_id,
            world_id,
            channel_id,
            transaction_id: operation_id,
            source_binding: digest.finalize().into(),
            item_instance_ids: vec![plan.item_instance_id],
        };
        let mut candidate = self
            .prepare_reward_claim_inbox_mint(
                authority,
                node,
                fence.item_fence(),
                &source,
                plan.mint.clone(),
            )
            .await
            .map_err(inbox_error)?;
        let write_plan = plan.clone();
        let current_node = node.clone();
        let mut outcome = None;
        let result = self
            .commit_reward_claim_inbox_mint(
                authority,
                node,
                fence.item_fence(),
                &mut candidate,
                &mut outcome,
                move |context, outcome| {
                    Box::pin(async move {
                        let tx = context.transaction();
                        let result = write_delivery(
                            tx,
                            &recovery,
                            &current_node,
                            operation_id,
                            &write_plan,
                            fence,
                        )
                        .await;
                        let accepted = result.is_ok();
                        *outcome = Some(result);
                        if accepted {
                            Ok(())
                        } else {
                            Err(RewardClaimInboxError::SourceRefused)
                        }
                    })
                },
            )
            .await;
        if result.is_ok()
            && let Some(receipt) = outcome
        {
            return receipt;
        }
        // A concurrent winner may have committed while this attempt waited on the
        // native command/root locks. Read its immutable receipt after rollback.
        if let Some(receipt) = self
            .read_store_delivery(authority, operation_id, &plan)
            .await?
        {
            return Ok(receipt);
        }
        if let Some(Err(error)) = outcome {
            return Err(error);
        }
        Err(result
            .err()
            .map(inbox_error)
            .unwrap_or(StoreDurabilityError::Unavailable(
                DurabilityError::InvalidStoredState,
            )))
    }

    async fn read_store_delivery(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        id: [u8; 16],
        plan: &DeliveryPlan,
    ) -> Result<Option<DeliveryReceipt>, StoreDurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let plan = plan.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let receipt = lookup_delivery(&mut tx, id, &plan).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(receipt)
                })
            })
            .await?
    }
}

fn inbox_error(
    error: crate::durability::character_inbox::RewardClaimInboxError,
) -> StoreDurabilityError {
    use crate::durability::character_inbox::RewardClaimInboxError;
    match error {
        RewardClaimInboxError::Unavailable(error) => error.into(),
        RewardClaimInboxError::AuthorityRejected => StoreDurabilityError::AuthorityRejected,
        RewardClaimInboxError::ConflictingDelivery => StoreDurabilityError::ConflictingOperation,
        other => StoreDurabilityError::Inbox(other),
    }
}

async fn account_matches(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterGameplayFence,
    account: &[u8; 16],
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT account_id=encode($2,'hex')::uuid FROM game_character_roots \
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(account.as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or(false))
}

async fn lookup_delivery(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: [u8; 16],
    plan: &DeliveryPlan,
) -> Result<Result<Option<DeliveryReceipt>, StoreDurabilityError>, DurabilityError> {
    let row = sqlx::query(
        "SELECT account_id::text,character_id::text,world_id::text,fulfilment_unit_id,receipt \
         FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid",
    )
    .bind(id.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(Ok(None));
    };
    if row.try_get::<String, _>("account_id")? != uuid_string(&plan.account_id)
        || row.try_get::<String, _>("character_id")? != uuid_string(&plan.character_id)
        || row.try_get::<String, _>("world_id")? != uuid_string(&plan.world_id)
        || row.try_get::<String, _>("fulfilment_unit_id")? != plan.fulfilment_unit_id
    {
        return Ok(Err(StoreDurabilityError::ConflictingOperation));
    }
    Ok(Ok(Some(
        serde_json::from_slice(&row.try_get::<Vec<u8>, _>("receipt")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    )))
}

async fn write_delivery(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    operation_id: [u8; 16],
    plan: &DeliveryPlan,
    fence: CurrentCharacterGameplayFence,
) -> Result<DeliveryReceipt, StoreDurabilityError> {
    assert_recovery_fence(tx, recovery).await?;
    if assert_gameplay_fence(tx, &fence, node).await?.is_err()
        || !account_matches(tx, &fence, &plan.account_id).await?
    {
        return Err(StoreDurabilityError::AuthorityRejected);
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-delivery:'||encode($1,'hex'),0))")
        .bind(operation_id.as_slice()).execute(&mut **tx).await?;
    if lookup_delivery(tx, operation_id, plan).await??.is_some() {
        // Roll back the losing native mint; the caller returns the stored receipt.
        return Err(StoreDurabilityError::ConflictingOperation);
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let receipt = DeliveryReceipt {
        operation_id: uuid_string(&operation_id),
        account_id: uuid_string(&plan.account_id),
        character_id: uuid_string(&plan.character_id),
        world_id: uuid_string(&plan.world_id),
        fulfilment_unit_id: plan.fulfilment_unit_id.clone(),
        result: "COMMITTED".into(),
        recovery_generation: recovery.recovery_generation.to_string(),
        decided_at: now.to_string(),
    };
    let bytes = encode_receipt(&receipt)?;
    sqlx::query(
        "INSERT INTO store_delivery_receipts \
         (operation_id,account_id,character_id,world_id,fulfilment_unit_id,result,\
          recovery_generation,decided_at,receipt,item_instance_id) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
                encode($4,'hex')::uuid,$5,'COMMITTED',$6::text::numeric,$7,$8,encode($9,'hex')::uuid)",
    )
    .bind(operation_id.as_slice())
    .bind(plan.account_id.as_slice())
    .bind(plan.character_id.as_slice())
    .bind(plan.world_id.as_slice())
    .bind(&plan.fulfilment_unit_id)
    .bind(recovery.recovery_generation.to_string())
    .bind(now)
    .bind(bytes)
    .bind(plan.item_instance_id.as_slice())
    .execute(&mut **tx)
    .await?;
    sqlx::query("INSERT INTO store_receipt_outbox(operation_id)VALUES(encode($1,'hex')::uuid)")
        .bind(operation_id.as_slice())
        .execute(&mut **tx)
        .await?;
    Ok(receipt)
}
