//! Discovery milestones use the same progress, fence and transaction as the final POI.
use super::storage::CanonicalProgress;
use super::{CurrentCharacterGameplayFence, DurabilityError, MapRequest, Type2Transaction};
use crate::durability::{account_achievement::grant_missing, character_appearance::lock_account_grants};
use oteryn_gameplay_quests::quest::reward_callbacks::PROVENANCE;
use sqlx::types::Uuid;
use std::collections::BTreeMap;

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

pub(super) async fn grant(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &MapRequest,
) -> Result<(), DurabilityError> {
    let character = Uuid::from_bytes(*fence.character_id.as_bytes());
    let text: String = sqlx::query_scalar(
        "SELECT progress::text FROM game_character_cyclopedia WHERE character_id=$1",
    )
    .bind(character)
    .fetch_one(&mut ***tx)
    .await?;
    let progress = CanonicalProgress::decode(&text)?;
    let mut mains = BTreeMap::new();
    for &(main, subarea) in &request.subareas {
        let discovered = progress.subareas.get(&subarea).is_some_and(|state| state.discovered);
        *mains.entry(main).or_insert(true) &= discovered;
    }
    if mains.len() > request.main_area_count {
        return Err(DurabilityError::InvalidStoredState);
    }
    // Crystal 201189f8 discovery_system.lua:528-559 counts only fully explored mains.
    let completed = mains.values().filter(|&&complete| complete).count();
    if completed == 0 {
        return Ok(());
    }
    let halfway = completed * 2 >= request.main_area_count;
    let all = completed == request.main_area_count;
    let outfit_count = 1 + usize::from(halfway) + usize::from(all);
    let achievement_count = 1 + usize::from(all);
    let account: Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=$1",
    )
    .bind(character)
    .fetch_one(&mut ***tx)
    .await?;
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
