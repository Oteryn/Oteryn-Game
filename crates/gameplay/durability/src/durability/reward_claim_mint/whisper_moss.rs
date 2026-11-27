//! Mission-one recovery requires both a committed missing-moss dialogue and
//! proof that the character's earlier chest issues were destroyed.
use super::RewardClaimMintRequest;
use crate::domain::CharacterId;
use sqlx::{Postgres, Transaction};

const CLAIM: &str = "oteryn:reward-claim.quest.u7_6.whisper_moss";
const REPLACEMENT: &str = "oteryn:reward-claim.quest.u7_6.whisper_moss/replacement/";
const ITEM: &str = "oteryn:item.tibia.i4827";
const QUEST: &str = "oteryn:quest.the_ape_city_quest";
const RECOVERY: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/whisper_moss_recovery";
const MISSION: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/questline";

#[cfg(test)]
mod tests;

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

async fn replacement_key(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
) -> Result<Option<String>, sqlx::Error> {
    // The latest missing-item reply names a one-shot claim. Committing
    // that claim mints the moss and consumes the reset in one transaction.
    sqlx::query_scalar(
        "SELECT concat($5::text, recovery.value::text)
         FROM game_character_quest_tracks mission
         JOIN game_character_quest_tracks recovery USING (character_id, quest_key)
         WHERE mission.character_id=encode($1,'hex')::uuid AND mission.quest_key=$2
           AND mission.track_key=$3 AND mission.value=1
           AND recovery.track_key=$4 AND recovery.value>0
           AND EXISTS (
             SELECT 1 FROM game_reward_claims
             WHERE character_id=mission.character_id AND claim_family='RewardClaim'
               AND claim_production_key=$6
           )",
    )
    .bind(character.as_bytes().as_slice())
    .bind(QUEST)
    .bind(MISSION)
    .bind(RECOVERY)
    .bind(REPLACEMENT)
    .bind(CLAIM)
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
    replacement_key(tx, character).await
}

pub(super) async fn permits(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    world: &[u8; 16],
    request: &RewardClaimMintRequest,
) -> Result<bool, sqlx::Error> {
    if !is_moss(request)
        || replacement_key(tx, character).await?.as_deref()
            != Some(request.claim.production_key.as_str())
    {
        return Ok(false);
    }
    // A merge or stash deposit can retire an identity while preserving its
    // value. Require a decay retirement receipt for each earlier chest issue.
    // Read returnable escrow and inbox together so settlement cannot hide a copy.
    sqlx::query_scalar(
        "WITH RECURSIVE roots(item_instance_id) AS (
           SELECT item_instance_id FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid
           UNION SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid
           UNION SELECT item_instance_id FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid
           UNION SELECT item_instance_id FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid
           UNION SELECT item_instance_id FROM game_item_character_depot_locations WHERE character_id=encode($1,'hex')::uuid
           UNION SELECT e.item_instance_id FROM game_market_offers o
             JOIN game_item_market_escrow_locations e USING (offer_id, world_id)
             WHERE o.character_id=encode($1,'hex')::uuid AND o.world_id=encode($5,'hex')::uuid
               AND o.definition_key=$2 AND o.side=2 AND o.state IN (1,5)
         ), custody(item_instance_id, depth) AS (
           SELECT item_instance_id, 0 FROM roots
           UNION ALL
           SELECT e.item_instance_id, c.depth+1 FROM custody c
           JOIN game_item_nested_entries e ON e.parent_item_instance_id=c.item_instance_id
           WHERE c.depth<16
         )
         SELECT NOT EXISTS (
           SELECT 1 FROM custody c JOIN game_item_instances i USING(item_instance_id)
           WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$2
         ) AND NOT EXISTS (
           SELECT 1 FROM game_character_stash_balances
           WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($5,'hex')::uuid
             AND definition_key=$2 AND quantity>0
         ) AND NOT EXISTS (
           SELECT 1 FROM game_reward_claims c
           WHERE c.character_id=encode($1,'hex')::uuid AND c.claim_family='RewardClaim'
             AND (c.claim_production_key=$3 OR starts_with(c.claim_production_key,$4))
             AND NOT EXISTS (
               SELECT 1 FROM game_reward_claim_mint_receipts r
               JOIN game_item_decay_retire_receipts d USING (item_instance_id)
               WHERE r.transaction_id=c.claimed_transaction_id
             )
         )",
    )
    .bind(character.as_bytes().as_slice())
    .bind(ITEM)
    .bind(CLAIM)
    .bind(REPLACEMENT)
    .bind(world.as_slice())
    .fetch_one(&mut **tx)
    .await
}
