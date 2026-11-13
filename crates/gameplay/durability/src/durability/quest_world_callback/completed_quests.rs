//! Read prerequisite completion under the existing character writer fence.
use super::{CurrentCharacterGameplayFence, DurabilityError};

pub(super) async fn require(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    quests: &[String],
) -> Result<bool, DurabilityError> {
    for quest in quests {
        let completed: Option<bool> = sqlx::query_scalar(
            "SELECT completed_character_revision IS NOT NULL \
             FROM game_character_quest_states \
             WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR SHARE",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(quest)
        .fetch_optional(&mut **tx)
        .await?;
        if completed != Some(true) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
#[path = "completed_quests_postgres_tests.rs"]
mod postgres_tests;
