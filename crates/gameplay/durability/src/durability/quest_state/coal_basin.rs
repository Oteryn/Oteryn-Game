//! The recipient half of the Ancient Tombs coin arrival. Runs inside the
//! item-move transaction, under the independently current recipient fence.
use super::*;
use crate::durability::{
    character_progression::assert_item_only_fence,
    item_transfer::CurrentCharacterItemFence,
};
use oteryn_gameplay_quests::quest::world_callbacks::coal_basins::{CoalBasin, QUEST, START, Treasure};

const TRANSITION: &str = "oteryn:quest-callback/ancient-tombs-coal-basin";

// Crystal 201189f8 movements_all_teleports_tombs_coal_basin.lua:7-19,44-55.
pub(in crate::durability) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    catalogue: &QuestStateCatalogue,
    basin: &CoalBasin,
    move_transaction: [u8; 16],
) -> Result<bool> {
    let root = assert_item_only_fence(tx, &fence, node).await??;
    if root.content_revision != catalogue.content_revision() {
        return Err(CharacterProgressionError::ProgressionContextMismatch);
    }
    let id = fence.character_id.as_bytes().as_slice();
    let current = sqlx::query(
        "SELECT character_revision::text,profile_revision,ruleset_revision,content_revision
         FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    ).bind(id).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    if numeric_u64(&current, "character_revision")? != root.revision
        || !state_matches_root(&current, &root)
    {
        return Err(CharacterProgressionError::ProgressionContextMismatch);
    }
    let hash = catalogue.definition_hash(QUEST).ok_or(CharacterProgressionError::InvalidInput)?;
    let state = sqlx::query(
        "SELECT pinned_content_revision,definition_hash FROM game_character_quest_states
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE",
    ).bind(id).bind(QUEST).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    let mut pin = catalogue.content_revision().to_owned();
    if let Some(state) = &state {
        let existing: Vec<u8> = state.try_get("definition_hash").map_err(DurabilityError::from)?;
        if existing != hash {
            return Err(CharacterProgressionError::ProgressionContextMismatch);
        }
        pin = state.try_get("pinned_content_revision").map_err(DurabilityError::from)?;
    }
    let keys: Vec<_> = std::iter::once(START).chain(basin.treasure.map(Treasure::track)).collect();
    let rows = sqlx::query(
        "SELECT track_key,quest_key,value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[])
         ORDER BY track_key FOR UPDATE",
    ).bind(id).bind(&keys).fetch_all(&mut **tx).await.map_err(DurabilityError::from)?;
    let mut stored = BTreeMap::new();
    for row in rows {
        let quest: String = row.try_get("quest_key").map_err(DurabilityError::from)?;
        if quest != QUEST {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let key: String = row.try_get("track_key").map_err(DurabilityError::from)?;
        let value: i64 = row.try_get("value").map_err(DurabilityError::from)?;
        stored.insert(key, value);
    }
    let mut changes = Vec::new();
    for key in keys {
        let track = catalogue.track(key).filter(|track| track.quest == QUEST)
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let before = stored.get(key).copied().unwrap_or(track.initial);
        if !(track.min..=track.max).contains(&before) || !(track.min..=track.max).contains(&1) {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        if (key == START && before != 1) || (key != START && before <= 0) {
            changes.push(QuestTrackChange { track: key.into(), before, after: 1 });
        }
    }
    if !changes.is_empty() {
        let keys: Vec<_> = changes.iter().map(|change| change.track.as_str()).collect();
        let existing = keys.iter().filter(|&&key| stored.contains_key(key)).count();
        if (state.is_none() && !capacity::new_state_fits(tx, fence.character_id).await?)
            || !capacity::tracks_fit(tx, fence.character_id, &keys, existing).await?
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let request = QuestTransitionRequest {
            transition_key: TRANSITION.into(),
            cause: QuestCause::SourceItemArrival(move_transaction),
        };
        let transition = oteryn_gameplay_quests::quest::QuestTransition {
            key: TRANSITION.into(), quest: QUEST.into(), world_change: None,
            effects: Vec::new(), completes: false, experience: None,
        };
        persist::persist(tx, persist::QuestWrite {
            character: fence.character_id, root_revision: root.revision,
            request: &request, transition: &transition, pin: &pin, hash: &hash,
            changes: &changes, binding: &request_binding(fence.character_id, &request),
            state_exists: state.is_some(),
        }).await?;
    }
    let world = fence.runtime_scope.world_id();
    let destination = basin.destination;
    sqlx::query(
        "INSERT INTO game_character_pending_arrivals(
            character_id,occurrence_id,world_id,tile_x,tile_y,tile_floor,source_coal_basin)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,true)",
    ).bind(id).bind(move_transaction.as_slice()).bind(world.as_bytes().as_slice())
      .bind(i32::from(destination.x)).bind(i32::from(destination.y))
      .bind(-i16::from(destination.floor)).execute(&mut **tx).await.map_err(DurabilityError::from)?;
    Ok(!changes.is_empty())
}
