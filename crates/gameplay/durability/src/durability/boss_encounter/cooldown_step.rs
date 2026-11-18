//! Exact read for a native boss staging portal, using the shared Character cooldown owner.
use super::{
    DurabilityError, NodeIncarnationProof, assert_recovery_fence,
    cooldown_storage::{BossRewardKey, read_boss_cooldown_data},
    lock_admission_relations,
};
use crate::durability::{
    DurabilityRoot,
    character_authority::ReconciledCharacterAuthority,
    character_progression::assert_item_only_fence,
    db::{begin_semantic_transaction, commit_semantic_transaction},
    item_transfer::CurrentCharacterItemFence,
    spell_owner_commit::SpellLanePermit,
};

impl DurabilityRoot {
    /// Returns remaining seconds at the transaction clock, or no decision for a stale fence.
    pub async fn boss_portal_cooldown_seconds(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        fence: CurrentCharacterItemFence,
        boss: &str,
    ) -> Result<Option<u64>, DurabilityError> {
        if permit.runtime_scope() != fence.runtime_scope || permit.has_unresolved() {
            return Ok(None);
        }
        let key = BossRewardKey::new(fence.character_id, boss)
            .map_err(|_| DurabilityError::CommitRejected)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::CommitRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_item_only_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(None);
                    }
                    let row = read_boss_cooldown_data(&mut tx, &key)
                        .await
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let now: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let remaining =
                        row.map_or(0, |row| row.available_at.saturating_sub(now).max(0));
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(remaining.unsigned_abs().div_ceil(1000)))
                })
            })
            .await
    }
}
