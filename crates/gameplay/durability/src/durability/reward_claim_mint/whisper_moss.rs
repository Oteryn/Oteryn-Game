//! Mission-one recovery preserves the original claim and binds each replacement
//! to the last granted moss instance. Freeze and commit check the same custody.
use super::RewardClaimMintRequest;
use crate::domain::CharacterId;
use sqlx::{Postgres, Transaction};

const CLAIM: &str = "oteryn:reward-claim.quest.u7_6.whisper_moss";
const REPLACEMENT: &str = "oteryn:reward-claim.quest.u7_6.whisper_moss/replacement/";
const ITEM: &str = "oteryn:item.tibia.i4827";
const QUEST: &str = "oteryn:quest.the_ape_city_quest";
const MISSION: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/questline";

fn is_moss(request: &RewardClaimMintRequest) -> bool {
    request.claim.family == "RewardClaim"
        && request.claim.revision_ref == "reward-claim-r1"
        && request.item.definition.family == "Item"
        && request.item.definition.production_key == ITEM
        && request.item.definition.revision_ref == "definition-r1"
        && request.quantity == 1
        && request.quest_transition.is_none()
        && request.achievement.is_none()
        && request.contents.is_empty()
        && request.additional_items.is_empty()
}

pub(super) fn is_original(request: &RewardClaimMintRequest) -> bool {
    request.claim.production_key == CLAIM && is_moss(request)
}

pub(super) fn is_replacement(key: &str) -> bool {
    key.starts_with(REPLACEMENT)
}

async fn last_mint(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT r.item_instance_id::text FROM game_reward_claims c
         JOIN game_reward_claim_mint_receipts r ON r.transaction_id=c.claimed_transaction_id
         WHERE c.character_id=encode($1,'hex')::uuid AND c.claim_family='RewardClaim'
           AND (c.claim_production_key=$2 OR starts_with(c.claim_production_key,$3))
           AND NOT EXISTS (
             SELECT 1 FROM game_reward_claims successor
             WHERE successor.character_id=c.character_id AND successor.claim_family=c.claim_family
               AND successor.claim_production_key=concat($3,r.item_instance_id::text)
           ) LIMIT 1",
    )
    .bind(character.as_bytes().as_slice())
    .bind(CLAIM)
    .bind(REPLACEMENT)
    .fetch_optional(&mut **tx)
    .await
}

pub(super) async fn resolve(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    stored: Option<String>,
) -> Result<Option<String>, sqlx::Error> {
    if let Some(key) = stored {
        return Ok(is_replacement(&key).then_some(key));
    }
    Ok(last_mint(tx, character).await?.map(|item| format!("{REPLACEMENT}{item}")))
}

pub(super) async fn permits(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    request: &RewardClaimMintRequest,
) -> Result<bool, sqlx::Error> {
    if !is_moss(request)
        || request.claim.production_key.strip_prefix(REPLACEMENT)
            != last_mint(tx, character).await?.as_deref()
    {
        return Ok(false);
    }
    // Any owned moss counts, including a copy kept in a nested bag or depot.
    // The item fence holds the Character root for this check and the mint.
    sqlx::query_scalar(
        "WITH RECURSIVE roots(item_instance_id) AS (
           SELECT item_instance_id FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid
           UNION SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid
           UNION SELECT item_instance_id FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid
           UNION SELECT item_instance_id FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid
           UNION SELECT item_instance_id FROM game_item_character_depot_locations WHERE character_id=encode($1,'hex')::uuid
         ), custody(item_instance_id, depth) AS (
           SELECT item_instance_id, 0 FROM roots
           UNION ALL
           SELECT e.item_instance_id, c.depth+1 FROM custody c
           JOIN game_item_nested_entries e ON e.parent_item_instance_id=c.item_instance_id
           WHERE c.depth<16
         )
         SELECT EXISTS (
           SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid
             AND quest_key=$2 AND track_key=$3 AND value=1
         ) AND EXISTS (
           SELECT 1 FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid
             AND claim_family='RewardClaim' AND claim_production_key=$4
         ) AND NOT EXISTS (
           SELECT 1 FROM custody c JOIN game_item_instances i USING(item_instance_id)
           WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$5
         )",
    )
    .bind(character.as_bytes().as_slice())
    .bind(QUEST)
    .bind(MISSION)
    .bind(CLAIM)
    .bind(ITEM)
    .fetch_one(&mut **tx)
    .await
}
