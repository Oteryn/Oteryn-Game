//! STAMINA-1 uses the existing build receipt and CharacterRevision writer.
use super::DurabilityRoot;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, apply_build_in_transaction,
};
use super::character_progression::{CharacterProgressionError, CurrentCharacterGameplayFence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::offline_training_state::{self, OfflineChange};
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::domain::CharacterId;
use sha2::{Digest, Sha256};

impl DurabilityRoot {
    /// Admission reads the canonical build column, including its absent-row seed.
    pub async fn read_hunting_stamina(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<u16, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let state = offline_training_state::load(&mut tx, character.as_bytes()).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(state.stamina_minutes))
                })
            })
            .await?
    }

    /// The caller holds this Character's revision sequencer. Retain the occurrence and
    /// before/after minutes until acknowledged; replay never reapplies the decrement.
    pub async fn commit_hunting_stamina(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        occurrence: BuildOccurrence,
        minutes: (u16, u16),
    ) -> Result<BuildCommitOutcome, CharacterProgressionError> {
        if minutes.0 > 2520 || minutes.1 >= minutes.0 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let replay: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_build_receipts WHERE build_occurrence_id=encode($1,'hex')::uuid)")
                .bind(occurrence.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !replay && let Err(error) = super::character_progression::assert_gameplay_fence(&mut tx, &fence, &node).await? { return Ok(Err(error)); }
            let row = sqlx::query("SELECT * FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let before = row.as_ref().map(|row| super::character_build::DurableBuildState::stored(row,"vocation", ""))
                .transpose()?.unwrap_or_default();
            let offline = offline_training_state::load(&mut tx, fence.character_id.as_bytes()).await?;
            let request = BuildChangeRequest { occurrence, cause: BuildCause::Training, after: before.clone(), before, pruned_stance: None };
            // Independent of a moving revision/build on retry: the occurrence freezes only
            // this decrease. The first write still checks the exact current revision.
            let mut binding = b"stamina-hunting-v1".to_vec();
            binding.extend_from_slice(fence.character_id.as_bytes());
            binding.extend_from_slice(&minutes.0.to_be_bytes());
            binding.extend_from_slice(&minutes.1.to_be_bytes());
            let content: String = sqlx::query_scalar("SELECT content_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let outcome = apply_build_in_transaction(&mut tx, &node, &fence, request,
                Sha256::digest(b"stamina-hunting-v1").into(), content, binding, Some(OfflineChange {
                    before: super::offline_training_state::OfflineTrainingState { stamina_minutes: minutes.0, ..offline },
                    after: super::offline_training_state::OfflineTrainingState { stamina_minutes: minutes.1, ..offline },
                })).await?;
            if outcome.is_ok() { commit_semantic_transaction(tx, deadline).await?; }
            Ok(outcome)
        })).await?
    }
}
