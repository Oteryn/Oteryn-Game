//! Arm and settle the exact source-committed personal return under the Character fence.
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, gated_binding};
use server_timer::QuestTimerOccurrence;

#[derive(Debug)]
pub struct DuePersonalReturn {
    pub occurrence: QuestTimerOccurrence,
    pub transition: String,
    pub already_committed: bool,
}

pub(super) async fn arm(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    map_revision: &str,
) -> StoredResult<()> {
    let Some(timer) = &callback.effects.personal_return else {
        return Ok(());
    };
    let key = callback.personal_return_key().ok_or(DurabilityError::InvalidStoredState)?;
    let binding = binding(callback)?;
    let rows = sqlx::query(
        "INSERT INTO game_quest_personal_returns(character_id, return_key, world_id, map_revision, binding, due_at)
         VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4,$5,clock_timestamp()+$6*interval '1 millisecond')
         ON CONFLICT(character_id,return_key) DO UPDATE SET
            occurrence=excluded.occurrence,world_id=excluded.world_id,map_revision=excluded.map_revision,
            binding=excluded.binding,due_at=excluded.due_at
         WHERE game_quest_personal_returns.committed_revision IS NULL",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(key)
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(map_revision)
    .bind(binding.as_slice())
    .bind(i32::try_from(timer.delay_ms).map_err(|_| DurabilityError::InvalidStoredState)?)
    .execute(&mut **tx)
    .await?;
    if rows.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

fn binding(callback: &Callback) -> StoredResult<[u8; 32]> {
    gated_binding(callback)
        .map_err(|_| DurabilityError::InvalidStoredState)?
        .ok_or(DurabilityError::InvalidStoredState)
}

pub(super) async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
) -> StoredResult<bool> {
    let QuestCause::ServerTimer(occurrence) = request.cause else {
        return Ok(false);
    };
    let ready: Option<bool> = sqlx::query_scalar(
        "SELECT true FROM game_quest_personal_returns
         WHERE character_id=encode($1,'hex')::uuid AND return_key=$2 AND occurrence=encode($3,'hex')::uuid
             AND world_id=encode($4,'hex')::uuid AND binding=$5
             AND due_at<=clock_timestamp() AND committed_revision IS NULL FOR UPDATE",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&request.transition_key)
    .bind(occurrence.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(binding(callback)?.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    Ok(ready == Some(true))
}

pub(super) async fn settle(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    revision: u64,
) -> StoredResult<()> {
    let QuestCause::ServerTimer(occurrence) = request.cause else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let rows = sqlx::query(
        "UPDATE game_quest_personal_returns SET committed_revision=$4::text::numeric,
            settled_xact_id=pg_current_xact_id()
         WHERE character_id=encode($1,'hex')::uuid AND return_key=$2 AND occurrence=encode($3,'hex')::uuid
            AND committed_revision IS NULL",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&request.transition_key)
    .bind(occurrence.as_bytes().as_slice())
    .bind(revision.to_string())
    .execute(&mut **tx)
    .await?;
    if rows.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

impl DurabilityRoot {
    pub async fn read_due_personal_return(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        map_revision: String,
    ) -> StoredResult<Option<DuePersonalReturn>> {
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            assert_gameplay_fence(&mut tx,&fence,&node).await?.map_err(|_| DurabilityError::Unavailable)?;
            let row = sqlx::query(
                "SELECT uuid_send(occurrence) AS occurrence,return_key,committed_revision IS NOT NULL AS committed
                 FROM game_quest_personal_returns
                 WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
                    AND map_revision=$3 AND due_at<=clock_timestamp()
                 ORDER BY due_at,return_key LIMIT 1",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .bind(map_revision)
            .fetch_optional(&mut *tx).await?;
            let due = row.map(|row| -> StoredResult<_> {
                let bytes: Vec<u8> = row.try_get("occurrence")?;
                Ok(DuePersonalReturn {
                    occurrence: QuestTimerOccurrence::from_bytes(bytes.try_into().map_err(|_| DurabilityError::InvalidStoredState)?)?,
                    transition: row.try_get("return_key")?,
                    already_committed: row.try_get("committed")?,
                })
            }).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(due)
        })).await
    }

    /// Once a settled return was published, or the guarded stage ended first,
    /// retire only this exact occurrence. A new attempt cannot be deleted by it.
    pub async fn finish_personal_return(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        due: DuePersonalReturn,
    ) -> StoredResult<()> {
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            super::super::character_progression::assert_item_only_fence(&mut tx,&fence.item_fence(),&node).await?.map_err(|_| DurabilityError::Unavailable)?;
            sqlx::query(
                "DELETE FROM game_quest_personal_returns WHERE character_id=encode($1,'hex')::uuid
                    AND return_key=$2 AND occurrence=encode($3,'hex')::uuid
                    AND world_id=encode($4,'hex')::uuid",
            )
            .bind(fence.character_id.as_bytes().as_slice()).bind(due.transition)
            .bind(due.occurrence.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(())
        })).await
    }
}
