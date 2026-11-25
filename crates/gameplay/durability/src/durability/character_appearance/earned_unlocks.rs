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
    // Token grants are local. A committed Platform delivery follows its Character
    // across Worlds while the purchasing Account still owns that Character.
    let store: Vec<String> = sqlx::query_scalar("SELECT g.offer_key FROM game_character_store_cosmetics g \
        JOIN game_cosmetic_token_purchases t USING(purchase_id,character_id,world_id) \
        WHERE g.character_id=encode($1,'hex')::uuid \
          AND g.world_id=(SELECT world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) \
          AND g.offer_key=ANY($2::text[]) AND g.operation_id=t.purchase_id \
        UNION \
        SELECT requested.offer_key FROM unnest($2::text[]) AS requested(offer_key) \
        JOIN store_delivery_receipts r ON r.entitlement_key='store-cosmetic:'||requested.offer_key \
        WHERE r.account_id=encode($3,'hex')::uuid AND r.character_id=encode($1,'hex')::uuid \
          AND r.entitlement_key LIKE 'store-cosmetic:%' \
        ORDER BY offer_key")
        .bind(character.as_bytes().as_slice()).bind(grant_keys).bind(account.as_bytes().as_slice()).fetch_all(connection).await?;
    facts.extend(store.into_iter().map(|key| {
        ((key, crate::content::store_cosmetics::PROVENANCE.into()), Source::Store)
    }));
    Ok(facts)
}
