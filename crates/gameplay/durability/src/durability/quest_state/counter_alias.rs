//! Canonical completion shares the helper's fenced counter receipt and revision.
use super::{DurabilityError, QuestTransitionRequest};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTransition, authored_bindings};
use sqlx::{Postgres, Row, Transaction};

/// The caller has already proved the helper's current counter cycle, and written
/// its receipt, scope and state under the Character root and World/counter locks.
pub(super) async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    transition: &QuestTransition,
    catalogue: &QuestStateCatalogue,
    revision: &str,
) -> Result<bool, DurabilityError> {
    let Some(quest) = authored_bindings::counter_canonical_quest(&transition.quest) else {
        return Ok(true);
    };
    let Some(hash) = catalogue.definition_hash(quest) else {
        return Ok(false);
    };
    let id = fence.character_id.as_bytes().as_slice();
    let state = sqlx::query("SELECT pinned_content_revision, definition_hash, completed_character_revision IS NOT NULL AS completed FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE")
        .bind(id).bind(quest).fetch_optional(&mut **tx).await?;
    let pin = if let Some(state) = &state {
        // Permanent history is never retagged by a later occurrence. The helper's
        // independently guarded scope still records every fresh completion.
        if state.try_get::<bool, _>("completed")? {
            return Ok(true);
        }
        if state.try_get::<Vec<u8>, _>("definition_hash")?.as_slice() != hash {
            return Ok(false);
        }
        state.try_get::<String, _>("pinned_content_revision")?
    } else {
        if !super::capacity::states_fit(tx, fence.character_id, 1).await? {
            return Ok(false);
        }
        catalogue.content_revision().to_owned()
    };
    let (cause, ordinal) = request.cause.key();
    sqlx::query("INSERT INTO game_quest_world_completions(character_id,cause_id,cause_ordinal,transition_key,quest_key,pinned_content_revision,definition_hash,committed_character_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,$7,$8::text::numeric)")
        .bind(id).bind(cause.as_slice()).bind(ordinal.to_string()).bind(&request.transition_key)
        .bind(quest).bind(&pin).bind(hash.as_slice()).bind(revision).execute(&mut **tx).await?;
    if state.is_some() {
        sqlx::query("UPDATE game_character_quest_states SET completed_character_revision=$3::text::numeric,committed_character_revision=$3::text::numeric WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
            .bind(id).bind(quest).bind(revision).execute(&mut **tx).await?;
    } else {
        sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,completed_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,$5::text::numeric,$5::text::numeric)")
            .bind(id).bind(quest).bind(&pin).bind(hash.as_slice()).bind(revision).execute(&mut **tx).await?;
    }
    Ok(true)
}
