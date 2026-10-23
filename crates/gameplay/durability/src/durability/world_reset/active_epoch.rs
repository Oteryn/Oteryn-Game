//! The active pointer is durable map identity, never inferred from an Item key.
use super::*;

impl DurabilityRoot {
    /// None means NotInitialized. Boot preserves epoch zero for a World which
    /// has never reset; calendar consumers must retain the missing disposition.
    pub async fn active_world_reset_epoch(&self, world: WorldId) -> Result<Option<u64>> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row: Option<(String, bool)> = sqlx::query_as("SELECT a.reset_epoch::text,a.reset_epoch=0 OR EXISTS(SELECT 1 FROM game_world_event_epochs e WHERE (e.world_id,e.reset_epoch)=(a.world_id,a.reset_epoch)) FROM game_world_reset_active a WHERE a.world_id=encode($1,'hex')::uuid")
                .bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let epoch = row.map(|(epoch, event_bound)| {
                if !event_bound { return Err(DurabilityError::InvalidStoredState); }
                epoch.parse().map_err(|_| DurabilityError::InvalidStoredState)
            }).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(epoch)
        })).await
    }
}
