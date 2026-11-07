use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence};
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use super::codec::{
    DeliveryIdentity, DeliveryPlan, DeliveryReceipt, StoreValidationError, encode_receipt, uuid_string, uuid_v7,
};
use sqlx::Row;
use super::hireling_model::HirelingStorePurchase;
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
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof,
        operation_id: [u8;16], plan: DeliveryPlan, fence: CurrentCharacterGameplayFence,
    ) -> Result<DeliveryReceipt, StoreDurabilityError> {
        self.apply_store_delivery_with_hireling(authority,node,operation_id,plan,fence,None).await
    }

    /// A lamp is minted by the same Inbox owner; its Hireling identity commits in that callback.
    pub async fn apply_store_hireling_delivery(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof,
        operation_id: [u8;16], plan: DeliveryPlan, fence: CurrentCharacterGameplayFence,
        purchase: HirelingStorePurchase,
    ) -> Result<DeliveryReceipt, StoreDurabilityError> {
        purchase.validate()?;
        if !matches!(purchase.action, super::hireling_model::HirelingStoreAction::Hire { .. }) { return Err(StoreDurabilityError::ConflictingOperation); }
        self.apply_store_delivery_with_hireling(authority,node,operation_id,plan,fence,Some(purchase)).await
    }

    async fn apply_store_delivery_with_hireling(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        operation_id: [u8; 16],
        plan: DeliveryPlan,
        fence: CurrentCharacterGameplayFence,
        hireling: Option<HirelingStorePurchase>,
    ) -> Result<DeliveryReceipt, StoreDurabilityError> {
        if !uuid_v7(&operation_id) {
            return Err(StoreValidationError::InvalidUuid.into());
        }
        plan.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        if let Some(receipt) = self
            .read_store_delivery(authority, operation_id, &plan, hireling.as_ref())
            .await?
        {
            return Ok(receipt);
        }
        let world_id=fence.runtime_scope.world_id();
        let channel_id=match fence.runtime_scope {
            crate::foundation::RuntimeScopeRefV1::Channel {channel_id,..}=>channel_id,
            crate::foundation::RuntimeScopeRefV1::Instance {..}=>crate::foundation::ChannelId::decode(&hireling.as_ref().ok_or(StoreDurabilityError::AuthorityRejected)?.origin_channel).map_err(|_|StoreDurabilityError::AuthorityRejected)?,
        };
        let origin_channel=hireling.as_ref().map(|p|p.origin_channel);
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
                    if !assert_store_fence(&mut tx, &fence, &current_node,origin_channel).await?
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
            // A rejected wire command is sequenced. Its native preparation remains durable,
            // so another real command needs its own attempt without rebinding that reservation.
            // The Platform operation, lamp identity and Store receipt remain unchanged.
            transaction_id: hireling.as_ref().map_or(operation_id, |_| hireling_attempt_id(operation_id,plan.command)),
            source_binding: digest.finalize().into(),
            item_instance_ids: vec![plan.item_instance_id],
        };
        let mut candidate = if hireling.is_some() {
            self.prepare_hireling_inbox_mint(authority,node,fence.item_fence(),&source,plan.mint.clone()).await
        } else {
            self.prepare_reward_claim_inbox_mint(authority,node,fence.item_fence(),&source,plan.mint.clone()).await
        }.map_err(inbox_error)?;
        let write_plan = plan.clone();
        let write_hireling = hireling.clone();
        let current_node = node.clone();
        let mut outcome = None;
        let result = if hireling.is_some() {
self
            .commit_hireling_inbox_mint(
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
                            write_hireling.as_ref(),
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
            .await
        } else {
self
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
                            write_hireling.as_ref(),
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
            .await
        };
        if result.is_ok()
            && let Some(receipt) = outcome
        {
            return receipt;
        }
        // A concurrent winner may have committed while this attempt waited on the
        // native command/root locks. Read its immutable receipt after rollback.
        if let Some(receipt) = self
            .read_store_delivery(authority, operation_id, &plan, hireling.as_ref())
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
        hireling: Option<&HirelingStorePurchase>,
    ) -> Result<Option<DeliveryReceipt>, StoreDurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
        let plan = plan.identity();
        let hireling = hireling.cloned();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let receipt = lookup_delivery(&mut tx, id, &plan, hireling.as_ref()).await?;
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

pub(super) async fn account_matches(
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

pub(super) async fn lookup_delivery(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: [u8; 16],
    plan: &DeliveryIdentity,
    purchase: Option<&HirelingStorePurchase>,
) -> Result<Result<Option<DeliveryReceipt>, StoreDurabilityError>, DurabilityError> {
    let row = sqlx::query(
        "SELECT account_id::text,character_id::text,world_id::text,fulfilment_unit_id,receipt,purchase_request_id::text \
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
        || row.try_get::<Option<String>, _>("purchase_request_id")? != purchase.map(|p| uuid_string(&p.purchase_request_id))
    {
        return Ok(Err(StoreDurabilityError::ConflictingOperation));
    }
    if let Some(purchase)=purchase {
        let action:Option<Vec<u8>>=sqlx::query_scalar("SELECT action FROM game_hireling_store_reservations WHERE purchase_request_id=encode($1,'hex')::uuid AND state=2 AND operation_id=encode($2,'hex')::uuid")
            .bind(purchase.purchase_request_id.as_slice()).bind(id.as_slice()).fetch_optional(&mut **tx).await?;
        let bytes=purchase.action.bytes().map_err(|_|DurabilityError::InvalidStoredState)?;
        if action.as_deref()!=Some(bytes.as_slice()) {return Ok(Err(StoreDurabilityError::ConflictingOperation))}
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
    hireling: Option<&HirelingStorePurchase>,
) -> Result<DeliveryReceipt, StoreDurabilityError> {
    assert_recovery_fence(tx, recovery).await?;
    if !assert_store_fence(tx, &fence, node,hireling.map(|p|p.origin_channel)).await?
        || !account_matches(tx, &fence, &plan.account_id).await?
    {
        return Err(StoreDurabilityError::AuthorityRejected);
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:store-delivery:'||encode($1,'hex'),0))")
        .bind(operation_id.as_slice()).execute(&mut **tx).await?;
    if lookup_delivery(tx, operation_id, &plan.identity(), hireling).await??.is_some() {
        // Roll back the losing native mint; the caller returns the stored receipt.
        return Err(StoreDurabilityError::ConflictingOperation);
    }
    if let Some(purchase) = hireling {
        super::hireling_reservation::apply_effect(tx,plan.character_id,operation_id,Some(plan.item_instance_id),purchase).await?;
    }
    let native_transaction=hireling.map(|_|hireling_attempt_id(operation_id,plan.command));
    write_receipt(tx,recovery,operation_id,&plan.identity(),Some(plan.item_instance_id),hireling,native_transaction).await
}

pub(super) async fn write_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    operation_id: [u8;16], plan: &DeliveryIdentity, item: Option<[u8;16]>,
    purchase: Option<&HirelingStorePurchase>,
    native_transaction: Option<[u8;16]>,
) -> Result<DeliveryReceipt,StoreDurabilityError> {
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
          recovery_generation,decided_at,receipt,item_instance_id,purchase_request_id,native_transaction_id) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
                encode($4,'hex')::uuid,$5,'COMMITTED',$6::text::numeric,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,encode($11,'hex')::uuid)",
    )
    .bind(operation_id.as_slice())
    .bind(plan.account_id.as_slice())
    .bind(plan.character_id.as_slice())
    .bind(plan.world_id.as_slice())
    .bind(&plan.fulfilment_unit_id)
    .bind(recovery.recovery_generation.to_string())
    .bind(now)
    .bind(bytes)
    .bind(item.as_ref().map(|id| id.as_slice()))
    .bind(purchase.map(|p| p.purchase_request_id.as_slice()))
    .bind(native_transaction.as_ref().map(|id|id.as_slice()))
    .execute(&mut **tx)
    .await?;
    sqlx::query("INSERT INTO store_receipt_outbox(operation_id)VALUES(encode($1,'hex')::uuid)")
        .bind(operation_id.as_slice())
        .execute(&mut **tx)
        .await?;
    Ok(receipt)
}

/// Native work is bounded per real CommandRef; a fresh wire retry does not reset or change an
/// earlier preparation. Stable Store operation/item identities still admit only one winner.
fn hireling_attempt_id(operation:[u8;16],command:crate::foundation::CommandRef)->[u8;16] {
    use sha2::{Digest,Sha256};
    let digest=Sha256::new().chain_update(b"oteryn:store-hireling-inbox-attempt:v1\0")
        .chain_update(operation).chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes()).finalize();
    let mut id=[0;16];id.copy_from_slice(&digest[..16]);
    id[6]=(id[6]&0x0f)|0x70;id[8]=(id[8]&0x3f)|0x80;id
}

/// The same current proof as ordinary Store, with the named-House adapter for Hireling offers.
pub(super) async fn assert_store_fence(
    tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,fence:&CurrentCharacterGameplayFence,
    node:&NodeIncarnationProof,origin:Option<[u8;16]>,
)->Result<bool,DurabilityError>{
    match fence.runtime_scope {
        crate::foundation::RuntimeScopeRefV1::Channel {channel_id,..}=>{
            if origin.is_some_and(|origin|origin!=*channel_id.as_bytes()){return Ok(false)}
            Ok(assert_gameplay_fence(tx,fence,node).await?.is_ok())
        }
        crate::foundation::RuntimeScopeRefV1::Instance {..}=>{
            let Some(origin)=origin else{return Ok(false)};
            if !super::super::house_hireling::fence::assert_house_inventory_fence(tx,node,&fence.item_fence(),origin).await?{return Ok(false)}
            Ok(sqlx::query_scalar::<_,bool>("SELECT character_revision=$2::text::numeric FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.expected_character_revision.get().to_string()).fetch_optional(&mut **tx).await?.unwrap_or(false))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{CommandId,CommandRef,GameSessionId};

    #[test]
    fn hireling_native_attempt_binds_full_command_and_preserves_exact_retry()->Result<(),String>{
        let uuid=|tag:u8| {let mut id=[tag;16];id[6]=0x70;id[8]=0x80;id};
        let command=|session:u8,command:u64| ->Result<CommandRef,String> {
            Ok(CommandRef::new(GameSessionId::decode(&uuid(session)).map_err(|e|format!("{e:?}"))?,
                CommandId::new(command).map_err(|e|format!("{e:?}"))?))
        };
        let operation=uuid(1);
        let original=hireling_attempt_id(operation,command(2,1)?);
        assert!(uuid_v7(&original));
        assert_ne!(original,operation);
        assert_eq!(original,hireling_attempt_id(operation,command(2,1)?));
        assert_ne!(original,hireling_attempt_id(operation,command(2,(1_u64<<40)+1)?));
        assert_ne!(original,hireling_attempt_id(operation,command(3,1)?));
        assert_ne!(original,hireling_attempt_id(uuid(4),command(2,1)?));
        Ok(())
    }
}
