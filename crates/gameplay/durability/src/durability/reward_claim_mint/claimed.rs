//! Read the existing once-claim owner; a gate never creates a shadow quest track.
use super::*;

impl DurabilityRoot {
    /// Resolve the destination definition for a canonical chest USE under the current
    /// fence. A replay rebuilds the original intent from the receipt's historical parent.
    /// The mint owner subsequently checks the complete binding, including source placement.
    pub async fn reward_claim_backpack_for_use(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        claim: TypedDefinitionRef,
    ) -> Result<TypedDefinitionRef> {
        let (world, channel) = scope_of(&fence).map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                lock_cause(&mut tx, command).await?;
                if !character_item_fence_is_current(
                    &mut tx, &node, &fence, command, fence.character_id,
                    *world.as_bytes(), *channel.as_bytes(),
                ).await? {
                    return Ok(Err(RewardClaimMintError::AuthorityRejected));
                }

                let backpack = if let Some(receipt) = load_receipt(&mut tx, command).await? {
                    if uuid_text(receipt.try_get("character_id")?)? != *fence.character_id.as_bytes()
                        || receipt.try_get::<String, _>("claim_family")? != claim.family
                        || receipt.try_get::<String, _>("claim_production_key")? != claim.production_key
                        || receipt.try_get::<String, _>("claim_revision_ref")? != claim.revision_ref
                    {
                        return Ok(Err(RewardClaimMintError::ConflictingCause));
                    }
                    let parent: String = receipt.try_get("destination_parent_item_instance_id")?;
                    let row: Option<(String, String, String)> = sqlx::query_as(
                        "SELECT definition_family, definition_production_key, definition_revision_ref \
                         FROM game_item_instances WHERE item_instance_id=$1::uuid AND world_id=encode($2,'hex')::uuid",
                    )
                    .bind(parent)
                    .bind(world.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let (family, production_key, revision_ref) =
                        row.ok_or(DurabilityError::InvalidStoredState)?;
                    TypedDefinitionRef { family, production_key, revision_ref }
                } else {
                    if is_claim_spent(&mut tx, fence.character_id, &claim.family, &claim.production_key).await? {
                        return Ok(Err(RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed)));
                    }
                    let Some(backpack) = load_slot(&mut tx, fence.character_id).await? else {
                        return Ok(Err(RewardClaimMintError::Refused(RewardClaimRefusal::NoMainBackpack)));
                    };
                    backpack.definition
                };
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(backpack))
            }))
            .await?
    }

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
                    let claimed = is_claim_spent(
                        &mut tx, fence.character_id, "RewardClaim", &claim,
                    ).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(claimed))
                })
            })
            .await?
    }
}

pub(super) async fn is_claim_spent(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    family: &str,
    key: &str,
) -> std::result::Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_reward_claims \
         WHERE character_id = encode($1,'hex')::uuid \
           AND claim_family = $2 AND claim_production_key = $3)",
    )
    .bind(character.as_bytes().as_slice())
    .bind(family)
    .bind(key)
    .fetch_one(&mut **tx)
    .await?)
}
