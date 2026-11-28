//! Character-owned quest tracker preferences, independent of quest progression.

use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{assert_item_only_fence, observe_item_only_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};

#[derive(Debug, Deserialize, Serialize)]
pub struct QuestTrackerEntry {
    pub quest_key: String,
    // Mission indices are local to the catalogue that projected the mission.
    pub mission: Option<(String, u32)>,
}

impl DurabilityRoot {
    /// Reads or replaces an ordered selection under the current character and connection fence.
    /// An admitted character with no saved preference reads an empty selection.
    pub async fn quest_tracker(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        mission_tracking: bool,
        selection: Option<Vec<QuestTrackerEntry>>,
    ) -> Result<Option<Vec<QuestTrackerEntry>>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let current = if selection.is_some() {
                        assert_item_only_fence(&mut tx, &fence, &node).await?
                    } else {
                        observe_item_only_fence(&mut tx, &fence, &node).await?
                    };
                    if current.is_err() {
                        return Ok(None);
                    }
                    let character = fence.character_id.as_bytes().as_slice();
                    let selection = if let Some(selection) = selection {
                        sqlx::query(
                            "INSERT INTO game_character_quest_tracker \
                             (character_id, mission_tracking, selection) VALUES (encode($1,'hex')::uuid,$2,$3) \
                             ON CONFLICT (character_id, mission_tracking) DO UPDATE \
                             SET selection = EXCLUDED.selection",
                        )
                        .bind(character)
                        .bind(mission_tracking)
                        .bind(Json(&selection))
                        .execute(&mut *tx)
                        .await?;
                        selection
                    } else {
                        sqlx::query_scalar::<_, Json<Vec<QuestTrackerEntry>>>(
                            "SELECT selection FROM game_character_quest_tracker \
                             WHERE character_id=encode($1,'hex')::uuid AND mission_tracking=$2",
                        )
                        .bind(character)
                        .bind(mission_tracking)
                        .fetch_optional(&mut *tx)
                        .await?
                        .map_or_else(Vec::new, |saved| saved.0)
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(selection))
                })
            })
            .await
    }
}
