//! Retention source work is independent of Platform publication configuration.
use super::*;

impl DurabilityRoot {
    /// Erase one bounded interval batch and queue its tombstones under current
    /// reconciled Character authority, even when publication is disabled.
    pub async fn maintain_public_online_statistics(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<u32> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let changed: i32 = sqlx::query_scalar("SELECT game_public_online_maintain(32)")
                        .fetch_one(&mut *tx)
                        .await?;
                    let changed = u32::try_from(changed)
                        .map_err(|_| super::super::DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(changed))
                })
            })
            .await?
    }
}
