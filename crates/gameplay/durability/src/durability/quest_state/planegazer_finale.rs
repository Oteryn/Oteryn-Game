//! The final source dialogue, never a raw Bestiary kill, grants the title atomically.
use super::super::{DurabilityError, character_progression::CurrentCharacterGameplayFence};
use super::{QuestCause, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::{QuestTrackChange, planegazer_finale as quest};
pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    changes: &[QuestTrackChange],
) -> Result<(), DurabilityError> {
    if request.transition_key != quest::COMPLETE {
        return Ok(());
    }
    if !matches!(request.cause, QuestCause::Command(_))
        || changes.len() != 2
        || !changes
            .iter()
            .any(|change| change.track == quest::READY && change.before == 1 && change.after == 1)
        || !changes
            .iter()
            .any(|change| change.track == quest::CLAIMED && change.after == 1)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("INSERT INTO game_character_titles(character_id,title,earned_at_unix_ms) VALUES(encode($1,'hex')::uuid,$2,floor(extract(epoch FROM statement_timestamp())*1000)::bigint) ON CONFLICT DO NOTHING")
        .bind(fence.character_id.as_bytes().as_slice()).bind(i16::from(oteryn_protocol_oteryn::character_info::CharacterTitle::Planegazer.id())).execute(&mut **tx).await?;
    Ok(())
}
