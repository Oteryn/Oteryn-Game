//! The active pointer is durable map identity, never inferred from an Item key.
use super::*;

impl DurabilityRoot {
    /// None means NotInitialized. Boot preserves epoch zero for a World which
    /// has never reset; calendar consumers must retain the missing disposition.
    /// This reads map identity independently of event state. Admission uses
    /// `active_world_event_epoch` to additionally require the event binding.
    pub async fn active_world_reset_epoch(&self, world: WorldId) -> Result<Option<u64>> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let epoch: Option<String> = sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid")
                .bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            epoch.map(|epoch| epoch.parse().map_err(|_| DurabilityError::InvalidStoredState)).transpose()
        })).await
    }

    /// PARITY-EVENT-CALLBACKS-1. None while the World is uninitialized or a reset is
    /// already RETIRING: the countdown never overlaps the reset it announces.
    pub async fn planned_world_reset(&self, world: WorldId) -> Result<Option<PlannedWorldReset>> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row: Option<(String, i64, i64)> = sqlx::query_as("WITH c AS (SELECT clock_timestamp() AS t) SELECT a.reset_epoch::text,floor(extract(epoch FROM ((date_trunc('day',(c.t AT TIME ZONE 'Europe/Berlin')-interval '10 hours')+interval '1 day 10 hours') AT TIME ZONE 'Europe/Berlin'))*1000)::bigint,floor(extract(epoch FROM c.t)*1000)::bigint FROM c,game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid AND e.calendar='TIBIA_SERVER_SAVE_BERLIN_10' AND NOT EXISTS(SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=a.world_id AND j.phase='RETIRING')")
                .bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            row.map(|(epoch, reset_at_unix_ms, database_now_unix_ms)| {
                Ok(PlannedWorldReset {
                    ending_epoch: epoch.parse().map_err(|_| DurabilityError::InvalidStoredState)?,
                    reset_at_unix_ms,
                    database_now_unix_ms,
                })
            }).transpose()
        })).await
    }
}
