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
}
