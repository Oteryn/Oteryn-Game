//! Read the existing once-claim owner; a gate never creates a shadow quest track.
use super::*;

impl DurabilityRoot {
    /// Current fenced presence of one exact RewardClaim. This read writes no row.
    pub async fn reward_claim_is_claimed(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        claim: String,
    ) -> Result<bool> {
        if !valid_quest_key(&claim) || !claim.starts_with("oteryn:reward-claim.") {
            return Err(RewardClaimMintError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if super::super::character_progression::assert_item_only_fence(
                        &mut tx, &fence, &node,
                    )
                    .await?
                    .is_err()
                    {
                        return Ok(Err(RewardClaimMintError::AuthorityRejected));
                    }
                    let claimed = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_reward_claims \
                         WHERE character_id=encode($1,'hex')::uuid \
                         AND claim_family='RewardClaim' AND claim_production_key=$2)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(claim)
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(claimed))
                })
            })
            .await?
    }
}
