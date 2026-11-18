//! Fenced channel-room admission uses the same cooldown rows as instanced bosses.
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::{CurrentCharacterGameplayFence, assert_item_only_fence};
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use super::cooldown_storage::{
    BossCooldownAdmissionData, BossCooldownChange, BossRewardKey, lock_reward_roots,
    set_boss_cooldowns_data,
};

impl DurabilityRoot {
    /// Commit a source room's cooldown only for a current player in the active reset.
    #[allow(
        clippy::too_many_arguments,
        reason = "the room source, reset and Character fence are independent proofs"
    )]
    pub async fn commit_channel_boss_cooldown(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        boss: &str,
        cooldown_s: u64,
        admission: [u8; 16],
        reset_epoch: u64,
        minimum_level: u32,
    ) -> Result<bool, DurabilityError> {
        if !matches!(
            fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ) {
            return Ok(false);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let key = BossRewardKey::new(fence.character_id, boss)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await?.is_err() {
                return Ok(false);
            }
            let reset: Option<String> = sqlx::query_scalar(
                "SELECT reset_epoch::text FROM game_world_reset_read_locked(encode($1,'hex')::uuid)",
            )
            .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .fetch_optional(&mut *tx)
            .await?;
            if reset.as_deref().and_then(|value| value.parse().ok()) != Some(reset_epoch) {
                return Ok(false);
            }
            let keys = [key];
            lock_reward_roots(&mut tx, &keys).await.map_err(|_| DurabilityError::InvalidStoredState)?;
            let meets_level: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_character_progression_state
                 WHERE character_id=encode($1,'hex')::uuid AND level >= $2
                   AND character_revision=$3::text::numeric)",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(i64::from(minimum_level))
            .bind(fence.expected_character_revision.get().to_string())
            .fetch_one(&mut *tx)
            .await?;
            if !meets_level {
                return Ok(false);
            }
            let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint")
                .fetch_one(&mut *tx).await?;
            let change = set_boss_cooldowns_data(&mut tx, &keys, &BossCooldownAdmissionData {
                admission, now, reset_epoch, cooldown_s, boosted: false,
            }).await.map_err(|_| DurabilityError::InvalidStoredState)?;
            if change != BossCooldownChange::Applied {
                return Ok(false);
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(true)
        })).await
    }
}
