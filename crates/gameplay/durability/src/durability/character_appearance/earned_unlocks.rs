//! Resolve earned facts for the account of the independently fenced Character.
use super::stored::{AppearanceReadError, read_account_appearance_unlocks};
use crate::domain::{AccountId, CharacterId};
use oteryn_protocol_oteryn::cyclopedia_character::gallery::owned::Source;
use sqlx::PgConnection;

type SourcedUnlock = ((String, String), Source);

pub(super) async fn read(
    connection: &mut PgConnection,
    character: CharacterId,
    grant_keys: &[String],
) -> Result<Vec<(String, String)>, AppearanceReadError> {
    Ok(read_with_sources(connection, character, grant_keys)
        .await?.into_iter().map(|(fact, _)| fact).collect())
}

pub(super) async fn read_with_sources(
    connection: &mut PgConnection,
    character: CharacterId,
    grant_keys: &[String],
) -> Result<Vec<SourcedUnlock>, AppearanceReadError> {
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(&mut *connection)
    .await?;
    let account = AccountId::from_bytes(*account.as_bytes())
        .map_err(|_| AppearanceReadError::InvalidStoredState)?;
    let mut facts: Vec<_> = read_account_appearance_unlocks(connection, account, grant_keys)
        .await?
        .into_iter()
        .filter_map(|fact| {
            if crate::content::store_cosmetics::offer(&fact.unlock_key).is_some() {
                return None;
            }
            let source = match fact.source_kind.as_str() {
                "quest" => Source::Quest,
                "npc" => Source::Npc,
                "mount_taming" => Source::Taming,
                "item_action" => Source::Item,
                "task_board" => Source::TaskBoard,
                _ => return None,
            };
            Some(((fact.unlock_key, fact.content_revision), source))
        }).collect();
    let store: Vec<String> = sqlx::query_scalar("SELECT offer_key FROM game_character_store_cosmetics WHERE character_id=encode($1,'hex')::uuid AND world_id=(SELECT world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) AND offer_key=ANY($2::text[]) AND operation_id IS NOT NULL ORDER BY offer_key LIMIT $3")
        .bind(character.as_bytes().as_slice()).bind(grant_keys)
        .bind(crate::content::store_cosmetics::MAX_OFFERS).fetch_all(connection).await?;
    facts.extend(store.into_iter().map(|key| {
        ((key, crate::content::store_cosmetics::PROVENANCE.into()), Source::Store)
    }));
    Ok(facts)
}
