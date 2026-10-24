//! Admitted timed-item commands. Command receipt and every item/fee write share one transaction.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CurrentCharacterGameplayFence, assert_gameplay_fence, uuid_text,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_fee_burn::FeeChangeFacts;
use super::item_mint_audit::check_uuid_v7;
use super::item_timed_state::{
    self as timed, TimedCheckpointRequest, TimedDefinitionFacts, TimedExpiryRequest,
    TimedExpiryTarget, TimedWriteError,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::GameSessionId;
use oteryn_protocol_oteryn::timed_item::{
    TimedDisposition, TimedItemResult, decode_timed_item_result, encode_timed_item_result,
};
use sqlx::{PgConnection, Row};

mod execute;
mod repair;
use execute::apply;

#[derive(Debug, Clone)]
pub enum TimedOperation {
    Inspect,
    Use {
        elapsed_ms: u64,
    },
    /// Server monotonic time only. The owner checks expiry each second and checkpoints at the accepted 60-second cadence.
    Advance(u64),
    Repair(Box<FeeChangeFacts>),
}

#[derive(Debug, Clone)]
pub struct TimedRequest {
    pub transaction_id: [u8; 16],
    pub content_digest: [u8; 32],
    pub event_id: [u8; 16],
    pub binding: [u8; 32],
    pub item: [u8; 16],
    pub expected_revision: u64,
    pub definition: TimedDefinitionFacts,
    pub target: Option<TimedExpiryTarget>,
    pub operation: TimedOperation,
    pub occurred_at_unix_ms: i64,
}

pub(super) type Result<T> = std::result::Result<T, TimedWriteError>;

pub(super) async fn receipt(
    connection: &mut PgConnection,
    session: GameSessionId,
    transaction: [u8; 16],
    binding: [u8; 32],
) -> Result<Option<TimedItemResult>> {
    let row = sqlx::query("SELECT request_binding,result FROM game_item_timed_commands WHERE transaction_id=encode($1,'hex')::uuid AND session_id=encode($2,'hex')::uuid")
        .bind(transaction.as_slice()).bind(session.as_bytes().as_slice()).fetch_optional(connection).await?;
    row.map(|row| {
        if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
            return Err(TimedWriteError::ConflictingWrite);
        }
        decode_timed_item_result(&row.try_get::<Vec<u8>, _>("result")?)
            .map_err(|_| DurabilityError::InvalidStoredState.into())
    })
    .transpose()
}

impl DurabilityRoot {
    /// A bounded lazy-row read under current gameplay authority; used by the owner clock.
    pub async fn read_timed_row(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        item: [u8; 16],
    ) -> Result<Option<timed::StoredTimedState>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| TimedWriteError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_gameplay_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(Err(TimedWriteError::AuthorityRejected));
                    }
                    let state = timed::load_timed_state(&mut tx, &item)
                        .await
                        .map_err(as_storage)?;
                    tx.commit(deadline).await?;
                    Ok(Ok(state))
                })
            })
            .await?
    }

    /// Replay before handle/content resolution, including committed-but-unacknowledged commands.
    pub async fn read_timed_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        session: GameSessionId,
        transaction: [u8; 16],
        binding: [u8; 32],
    ) -> Result<Option<TimedItemResult>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| TimedWriteError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    match receipt(&mut tx, session, transaction, binding).await {
                        Ok(value) => {
                            tx.commit(deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(TimedWriteError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    /// Atomic consumption/expiry/repair, under the existing recovery, session, lease and scope fence.
    pub async fn commit_timed_action(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: TimedRequest,
    ) -> Result<TimedItemResult> {
        if check_uuid_v7(&request.transaction_id).is_err()
            || check_uuid_v7(&request.event_id).is_err()
            || check_uuid_v7(&request.item).is_err()
            || request.occurred_at_unix_ms <= 0
        {
            return Err(TimedWriteError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| TimedWriteError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex'),158))")
                .bind(request.transaction_id.as_slice()).execute(&mut **tx).await?;
            match receipt(&mut tx, fence.game_session_id, request.transaction_id, request.binding).await {
                Ok(Some(result)) => { tx.commit(deadline).await?; return Ok(Ok(result)); },
                Ok(None) => (),
                Err(TimedWriteError::Unavailable(error)) => return Err(error),
                Err(error) => return Ok(Err(error)),
            }
            if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err() { return Ok(Err(TimedWriteError::AuthorityRejected)); }
            let item_fence = fence.item_fence();
            if super::character_equipment::assert_current_equipment_in_transaction(&mut tx,&recovery,&node,&item_fence,None,request.content_digest).await.is_err() {
                return Ok(Err(TimedWriteError::AuthorityRejected));
            }
            let result = match apply(&mut tx, &fence, &request).await {
                Ok(result) => result,
                Err(TimedWriteError::Unavailable(error)) => return Err(error),
                Err(error) => return Ok(Err(error)),
            };
            let bytes = encode_timed_item_result(result).map_err(|_| DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_item_timed_commands(transaction_id,session_id,character_id,item_instance_id,request_binding,result) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6)")
                .bind(request.transaction_id.as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(request.item.as_slice()).bind(request.binding.as_slice()).bind(bytes).execute(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(Ok(result))
        })).await?
    }
}

fn as_storage(error: TimedWriteError) -> DurabilityError {
    match error {
        TimedWriteError::Unavailable(error) => error,
        _ => DurabilityError::InvalidStoredState,
    }
}
