//! Bounded periodic house-rent worker.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::WorldId;

impl DurabilityRoot {
    pub async fn advance_house_rent(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: WorldId,
    ) -> Result<bool, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let changed: bool = sqlx::query_scalar(
                        "SELECT game_house_rent_tick(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4)",
                    )
                    .bind(world.as_bytes().as_slice())
                    .bind(node.fact().node_id().as_bytes().as_slice())
                    .bind(node.fact().registration_revision().to_string())
                    .bind(node.sql_possession_secret())
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(changed)
                })
            })
            .await
    }
}
