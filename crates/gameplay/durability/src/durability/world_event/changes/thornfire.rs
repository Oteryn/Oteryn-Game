use super::*;
use crate::durability::character_progression::assert_runtime_scope_fence;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_world_change::thornfire::{COMPLETE_PROGRESS, at_save};

/// Captures the request for this epoch in the reset finalization transaction.
pub(in crate::durability::world_event) async fn record_save(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<(), DurabilityError> {
    let progress: Option<i64> = sqlx::query_scalar(
        "SELECT elf_homes FROM game_thornfire_progress WHERE world_id=encode($1,'hex')::uuid AND reset_epoch<$2::text::numeric FOR UPDATE",
    )
    .bind(world.as_bytes().as_slice())
    .bind(epoch.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let (progress, requested) = at_save(progress.unwrap_or_default());
    sqlx::query("INSERT INTO game_thornfire_progress(world_id,reset_epoch,elf_homes,requested_at_save) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch,elf_homes=EXCLUDED.elf_homes,requested_at_save=EXCLUDED.requested_at_save WHERE game_thornfire_progress.reset_epoch<EXCLUDED.reset_epoch")
        .bind(world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .bind(progress)
        .bind(requested)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

impl DurabilityRoot {
    /// A native credited kill marks one exact spawn home. Replays and respawns from
    /// the same home are idempotent within the current collection period.
    pub async fn credit_thornfire_elf(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        epoch: u64,
        home_bit: i64,
    ) -> Result<(), DurabilityError> {
        if !matches!(scope, RuntimeScopeRefV1::Channel { .. })
            || !(1..=COMPLETE_PROGRESS).contains(&home_bit)
            || home_bit.count_ones() != 1
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                return Ok(());
            }
            sqlx::query("INSERT INTO game_thornfire_progress(world_id,reset_epoch,elf_homes,requested_at_save)
                SELECT a.world_id,a.reset_epoch,$3,false FROM game_world_reset_active a
                WHERE a.world_id=encode($1,'hex')::uuid AND a.reset_epoch=$2::text::numeric
                AND NOT EXISTS (SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=a.world_id AND j.phase='RETIRING')
                ON CONFLICT(world_id) DO UPDATE SET elf_homes=game_thornfire_progress.elf_homes | EXCLUDED.elf_homes
                WHERE game_thornfire_progress.reset_epoch=EXCLUDED.reset_epoch")
                .bind(scope.world_id().as_bytes().as_slice())
                .bind(epoch.to_string())
                .bind(home_bit)
                .execute(&mut *tx)
                .await?;
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    /// The burning-phase owner can consume the decision captured for the active save.
    pub async fn thornfire_requested_at_save(
        &self,
        world: WorldId,
        epoch: u64,
    ) -> Result<bool, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let requested: Option<bool> = sqlx::query_scalar("SELECT p.requested_at_save FROM game_thornfire_progress p JOIN game_world_reset_active a USING(world_id,reset_epoch) WHERE p.world_id=encode($1,'hex')::uuid AND p.reset_epoch=$2::text::numeric")
                .bind(world.as_bytes().as_slice())
                .bind(epoch.to_string())
                .fetch_optional(&mut *tx)
                .await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(requested.unwrap_or_default())
        })).await
    }
}
