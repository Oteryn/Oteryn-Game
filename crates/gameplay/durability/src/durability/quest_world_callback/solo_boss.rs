//! Level and cooldown are qualified under the existing Character and boss ledgers.
use super::*;
use crate::durability::{
    DurabilityRoot,
    boss_encounter::cooldown_storage::{
        BossCooldownAdmissionData, BossCooldownChange, BossRewardKey, read_boss_cooldown_data,
        set_boss_cooldowns_data,
    },
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::{CharacterProgressionError, assert_gameplay_fence},
    db::{begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations},
    runtime_scope_assignment::NodeIncarnationProof,
};
use crate::foundation::CommandRef;
use oteryn_gameplay_quests::quest::world_callbacks::Trigger;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[allow(
    clippy::too_many_arguments,
    reason = "the admission binds a current player and the loaded physical StepIn"
)]
impl DurabilityRoot {
    pub async fn commit_solo_boss_step(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        callback_key: &str,
        catalogue: Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>,
        permit: &SpellLanePermit,
        context: QuestWorldContext,
    ) -> Result<bool, CharacterProgressionError> {
        use CharacterProgressionError as Error;
        if command.game_session_id() != fence.game_session_id {
            return Err(Error::InvalidInput);
        }
        let callback = oteryn_gameplay_quests::quest::world_callbacks::load_for(&catalogue)
            .map_err(|_| Error::InvalidInput)?
            .into_iter()
            .find(|callback| callback.key == callback_key && callback.trigger == Trigger::StepIn)
            .ok_or(Error::InvalidInput)?;
        let encounter = callback
            .effects
            .encounter
            .as_ref()
            .ok_or(Error::InvalidInput)?;
        let seconds = encounter.boss_cooldown_seconds.ok_or(Error::InvalidInput)?;
        let [boss] = encounter.monsters.as_slice() else {
            return Err(Error::InvalidInput);
        };
        let key = BossRewardKey::new(fence.character_id, &boss.creature)
            .map_err(|_| Error::InvalidInput)?;
        let digest = Sha256::new()
            .chain_update(b"solo-boss-step-v1")
            .chain_update(command.game_session_id().as_bytes())
            .chain_update(command.command_id().get().to_be_bytes())
            .chain_update(callback_key.as_bytes())
            .finalize();
        let mut admission = [0; 16];
        admission.copy_from_slice(&digest[..16]);
        admission[6] = (admission[6] & 0x0f) | 0x70;
        admission[8] = (admission[8] & 0x3f) | 0x80;
        let minimum = i64::from(encounter.minimum_level);
        let recovery = authority
            .record_for(self)
            .map_err(|_| Error::AuthorityRejected)?;
        let node = node.clone();
        let mut participants = (permit, context);
        self.try_issue_semantic_pass()?.run_with_context(&mut participants, move |holder, deadline, (permit, context)| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                    return Ok(Err(error));
                }
                if !context_is_current(&mut tx, permit, fence, context).await? {
                    return Ok(Ok(false));
                }
                let id = fence.character_id.as_bytes().as_slice();
                let eligible: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_character_progression_state s
                     WHERE s.character_id=encode($1,'hex')::uuid AND s.level >= $2
                       AND s.content_revision=$3 AND NOT EXISTS(
                       SELECT FROM game_character_pending_respawns p WHERE p.character_id=s.character_id))",
                ).bind(id).bind(minimum).bind(catalogue.content_revision()).fetch_one(&mut *tx).await?;
                if !eligible {
                    return Ok(Ok(false));
                }
                let old = read_boss_cooldown_data(&mut tx, &key).await
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                if old.is_some_and(|old| old.admission == admission && old.reset_epoch == context.reset_epoch) {
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(Ok(true));
                }
                let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
                    .fetch_one(&mut *tx).await?;
                let outcome = set_boss_cooldowns_data(&mut tx, &[key], &BossCooldownAdmissionData {
                    admission, now, reset_epoch: context.reset_epoch, cooldown_s: u64::from(seconds), boosted: false,
                }).await.map_err(|error| match error {
                    crate::durability::boss_encounter::cooldown_storage::BossRewardStateError::Unavailable(error) => error,
                    _ => DurabilityError::InvalidStoredState,
                })?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(outcome != BossCooldownChange::OnCooldown))
            })
        }).await?
    }
}
