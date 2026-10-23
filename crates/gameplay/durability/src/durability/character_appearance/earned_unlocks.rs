//! Resolve earned facts for the account of the independently fenced Character.
use super::stored::{AppearanceReadError, read_account_appearance_unlocks};
use crate::domain::{AccountId, CharacterId};
use sqlx::PgConnection;

pub(super) async fn read(
    connection: &mut PgConnection,
    character: CharacterId,
    grant_keys: &[String],
) -> Result<Vec<(String, String)>, AppearanceReadError> {
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(&mut *connection)
    .await?;
    let account = AccountId::from_bytes(*account.as_bytes())
        .map_err(|_| AppearanceReadError::InvalidStoredState)?;
    Ok(
        read_account_appearance_unlocks(connection, account, grant_keys)
            .await?
            .into_iter()
            .filter(|fact| matches!(fact.source_kind.as_str(), "quest" | "npc"))
            .map(|fact| (fact.unlock_key, fact.content_revision))
            .collect(),
    )
}
