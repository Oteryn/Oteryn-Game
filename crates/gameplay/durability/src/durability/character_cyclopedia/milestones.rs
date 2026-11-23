//! Discovery milestones reconcile in the fenced discovery transaction.
use super::storage::CanonicalProgress;
use super::{CurrentCharacterGameplayFence, DurabilityError, MapRequest, Type2Transaction};
use crate::durability::{
    account_achievement::grant_missing, character_appearance::lock_account_grants,
};
use oteryn_gameplay_quests::quest::reward_callbacks::PROVENANCE;
use sqlx::types::Uuid;

// These existing earned mappings each cover both Discoverer appearances (1094/1095).
const DISCOVERER: [&str; 3] = [
    "oteryn:outfit.discoverer/quest-grant/0",
    "oteryn:outfit.discoverer/quest-grant/1",
    "oteryn:outfit.discoverer/quest-grant/2",
];
const ACHIEVEMENTS: [&str; 2] = [
    "oteryn:achievement/widely_travelled",
    "oteryn:achievement/measuring_the_world",
];

pub(super) async fn reconcile(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &MapRequest,
) -> Result<(), DurabilityError> {
    if request.milestone_subareas.is_empty() {
        return Ok(());
    }
    let character = Uuid::from_bytes(*fence.character_id.as_bytes());
    let row: Option<(String, Uuid, Vec<String>, Vec<String>)> = sqlx::query_as(
        "SELECT c.progress::text, r.account_id, \
           ARRAY(SELECT unlock_key FROM game_account_appearance_unlocks \
                 WHERE account_id=r.account_id AND unlock_key=ANY($2)), \
           ARRAY(SELECT achievement_key FROM game_account_achievements \
                 WHERE account_id=r.account_id AND achievement_key=ANY($3)) \
         FROM game_character_cyclopedia c JOIN game_character_roots r USING(character_id) \
         WHERE c.character_id=$1",
    )
    .bind(character)
    .bind(DISCOVERER.as_slice())
    .bind(ACHIEVEMENTS.as_slice())
    .fetch_optional(&mut ***tx)
    .await?;
    let Some((text, account, outfits, achievements)) = row else {
        return Ok(());
    };
    let progress = CanonicalProgress::decode(&text)?;
    let mains = progress.completion(&request.milestone_subareas).mains();
    if mains.completed == 0 {
        return Ok(());
    }
    let halfway = mains.completed * 2 >= mains.total;
    let all = mains.is_complete();
    let outfit_count = 1 + usize::from(halfway) + usize::from(all);
    let achievement_count = 1 + usize::from(all);
    if DISCOVERER[..outfit_count]
        .iter()
        .all(|key| outfits.iter().any(|outfit| outfit == key))
        && ACHIEVEMENTS[..achievement_count]
            .iter()
            .all(|key| achievements.iter().any(|achievement| achievement == key))
    {
        return Ok(());
    }
    lock_account_grants(tx, account).await?;
    // Reuse the admitted quest-style outfit mappings and namespace the discovery event.
    let mut event = b"oteryn:cyclopedia-discovery:v1\0".to_vec();
    event.extend_from_slice(&request.occurrence);
    sqlx::query(
        "WITH grants AS ( \
           INSERT INTO game_account_appearance_unlock_receipts \
             (grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) \
           SELECT gen_random_uuid(),$1,key,$2,$3,'quest',$4 FROM unnest($5::text[]) AS keys(key) \
           WHERE NOT EXISTS (SELECT 1 FROM game_account_appearance_unlocks WHERE account_id=$1 AND unlock_key=key) \
           RETURNING account_id,unlock_key,grant_occurrence_id \
         ) INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) \
           SELECT account_id,unlock_key,grant_occurrence_id FROM grants",
    )
    .bind(account)
    .bind(PROVENANCE)
    .bind(character)
    .bind(event)
    .bind(&DISCOVERER[..outfit_count])
    .execute(&mut ***tx)
    .await?;
    grant_missing(
        tx,
        fence.character_id,
        &ACHIEVEMENTS[..achievement_count],
        "oteryn:cyclopedia-discovery",
        &request.occurrence,
    )
    .await
}
