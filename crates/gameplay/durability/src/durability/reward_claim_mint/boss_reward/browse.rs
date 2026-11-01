use super::*;
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use sqlx::Row;

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn read_boss_reward_items(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        death: [u8; 16],
        after_ordinal: u16,
        content_digest: [u8; 32],
    ) -> Result<Option<BossRewardPage>, DurabilityError> {
        if death == [0; 16] || after_ordinal > 1_000 {
            return Ok(None);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !admit(&mut tx, &node, &fence, content_digest).await? { return Ok(None); }
            let now = store::now(&mut tx).await?;
            store::expire(&mut tx, fence, now).await?;
            let expiry: Option<i64> = sqlx::query_scalar("SELECT expires_at FROM game_character_boss_reward_chests WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND death_id=encode($3,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(death.as_slice()).fetch_optional(&mut *tx).await?;
            let mut page = BossRewardPage { status: BossRewardCollectionStatus::Empty, death, expires_at_unix_ms: 0, items: vec![], more: false };
            if let Some(expiry) = expiry {
                page.expires_at_unix_ms = u64::try_from(expiry).map_err(|_| DurabilityError::InvalidStoredState)?;
                if now >= expiry {
                    page.status = BossRewardCollectionStatus::Expired;
                } else {
                    let rows = sqlx::query("SELECT item_ordinal,definition_family,definition_production_key,definition_revision_ref,quantity FROM game_boss_reward_items WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND death_id=encode($3,'hex')::uuid AND item_ordinal>$4 AND collected_transaction_id IS NULL AND NOT expired ORDER BY item_ordinal LIMIT 65")
                        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(death.as_slice()).bind(i32::from(after_ordinal)).fetch_all(&mut *tx).await?;
                    page.more = rows.len() > BOSS_REWARD_PAGE_SIZE;
                    for row in rows.into_iter().take(BOSS_REWARD_PAGE_SIZE) {
                        page.items.push(BossRewardPageItem {
                            ordinal: u16::try_from(row.try_get::<i32, _>("item_ordinal")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                            definition: TypedDefinitionRef { family: row.try_get("definition_family")?, production_key: row.try_get("definition_production_key")?, revision_ref: row.try_get("definition_revision_ref")? },
                            quantity: u32::try_from(row.try_get::<i64, _>("quantity")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                        });
                    }
                    if !page.items.is_empty() { page.status = BossRewardCollectionStatus::Collected; }
                }
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Some(page))
        })).await
    }
}
