//! Read completion from registered native owners; never mirror their state.
use oteryn_gameplay_quests::quest::rotten_blood::QUEST;

/// A native false result must not fall back to a generic completion row.
/// The calling Quest writer already holds this Character's root lock, shared
/// with the native owner; this read introduces no additional write or lock.
pub(super) async fn completed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: &[u8],
    quest: &str,
) -> super::StoredResult<Option<bool>> {
    if quest != QUEST {
        return Ok(None);
    }
    let completed = sqlx::query_scalar(
        "SELECT coalesce((SELECT completed FROM game_character_rotten_blood \
         WHERE character_id=encode($1,'hex')::uuid),false)",
    )
    .bind(character)
    .fetch_one(&mut **tx)
    .await?;
    Ok(Some(completed))
}
