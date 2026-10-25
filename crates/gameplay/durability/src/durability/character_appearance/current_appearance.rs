//! Current appearance state loaded under recovery, admission and Character fences.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentCharacterAppearance {
    pub sex: Sex,
    pub selected: Option<DurableAppearanceSelection>,
    pub earned: Vec<(String, String)>,
}

impl DurabilityRoot {
    /// Read stored selection DATA under the current native DB gameplay fence.
    /// This proves neither active content/sex compatibility nor current runtime actor/condition
    /// ownership. Callers must independently recheck those owners before publication; selection
    /// provenance may deliberately differ from the currently admitted content revision.
    pub async fn read_current_character_appearance(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<DurableAppearanceSelection>, AppearanceWriteError> {
        self.read_current_character_appearance_state(authority, node, fence, Vec::new())
            .await
            .map(|state| state.selected)
    }

    pub async fn read_current_character_appearance_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        grant_keys: Vec<String>,
    ) -> Result<CurrentCharacterAppearance, AppearanceWriteError> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(AppearanceWriteError::InvalidInput);
        }
        let recovery = authority.record_for(self).map_err(|_| {
            AppearanceWriteError::Fence(CharacterProgressionError::AuthorityRejected)
        })?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    let state = sqlx::query("SELECT character_revision::text, profile_revision, ruleset_revision, content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                        .bind(fence.character_id.as_bytes().as_slice())
                        .fetch_optional(&mut *tx).await?;
                    let Some(state) = state else {
                        return Ok(Err(CharacterProgressionError::MissingProgressionState.into()));
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if !state_matches_root(&state, &root) {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch.into()));
                    }
                    let selected = read_selected_appearance(&mut tx, fence.character_id)
                        .await.map_err(read_storage_error)?;
                    if selected.as_ref().is_some_and(|selected| selected.committed_character_revision.get() > root.revision) {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                    if pending {
                        return Ok(Err(CharacterProgressionError::RespawnPending.into()));
                    }
                    let sex = match character_sex::read(&mut tx, fence.character_id).await {
                        Ok(sex) => sex,
                        Err(error) => return Ok(Err(error)),
                    };
                    let earned = earned_unlocks::read(&mut tx, fence.character_id, &grant_keys).await.map_err(read_storage_error)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(CurrentCharacterAppearance { sex, selected, earned }))
                })
            })
            .await?
    }
}
