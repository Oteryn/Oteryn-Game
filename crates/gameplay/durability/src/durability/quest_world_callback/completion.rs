//! A child completion shares the admitted native reward's fence, receipt and transaction.
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::durability::{DurabilityError, character_progression::CurrentCharacterGameplayFence};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    world_callbacks::{self, Callback},
};
use sqlx::{Postgres, Row, Transaction};

/// The caller has written the primary receipt/state under the locked Character root.
/// Returning false requires rolling back the entire reward transaction.
pub(in crate::durability) async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
    catalogue: &QuestStateCatalogue,
) -> Result<bool, DurabilityError> {
    let Some(quest) = callback
        .effects
        .quest_completion
        .as_ref()
        .and_then(|completion| completion.child_quest())
    else {
        return Ok(true);
    };
    let QuestCause::Use(command) = request.cause else {
        return Ok(false);
    };
    let canonical = world_callbacks::completion_transition(callback)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if catalogue.transition(&canonical.key) != Some(&canonical) {
        return Ok(false);
    }
    let hash = catalogue
        .definition_hash(quest)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let id = fence.character_id.as_bytes().as_slice();
    let revision = fence
        .expected_character_revision
        .get()
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?
        .to_string();
    let state = sqlx::query("SELECT pinned_content_revision, definition_hash, completed_character_revision IS NOT NULL AS completed FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE")
        .bind(id).bind(quest).fetch_optional(&mut **tx).await?;
    let pin = if let Some(state) = &state {
        if state.try_get::<bool, _>("completed")?
            || state.try_get::<Vec<u8>, _>("definition_hash")?.as_slice() != hash
        {
            return Ok(false);
        }
        state.try_get::<String, _>("pinned_content_revision")?
    } else {
        if !crate::durability::quest_state::capacity::states_fit(tx, fence.character_id, 1).await? {
            return Ok(false);
        }
        catalogue.content_revision().to_owned()
    };
    sqlx::query("INSERT INTO game_quest_world_completions(character_id,cause_id,cause_ordinal,transition_key,quest_key,pinned_content_revision,definition_hash,committed_character_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,$7,$8::text::numeric)")
        .bind(id).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .bind(&callback.key).bind(quest).bind(&pin).bind(hash.as_slice()).bind(&revision).execute(&mut **tx).await?;
    if state.is_some() {
        sqlx::query("UPDATE game_character_quest_states SET completed_character_revision=$3::text::numeric,committed_character_revision=$3::text::numeric WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
            .bind(id).bind(quest).bind(&revision).execute(&mut **tx).await?;
    } else {
        sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,completed_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,$5::text::numeric,$5::text::numeric)")
            .bind(id).bind(quest).bind(&pin).bind(hash.as_slice()).bind(&revision).execute(&mut **tx).await?;
    }
    Ok(true)
}
