//! Character receipt for the storages written by the source basin teleport.
use super::*;
use crate::durability::character_progression::FencedCharacterRoot;
use crate::durability::quest_state::capacity;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;

const QUEST: &str = "oteryn:quest.the_ancient_tombs_quest";
const START: &str = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/default_start";
const DIPHRAH: &str = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/diphtrahs_treasure";
const MAHRDIS: &str = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/mahrdis_treasure";

pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    catalogue: &QuestStateCatalogue,
    character: CharacterId,
    root: &FencedCharacterRoot,
    command: CommandRef,
    uid: u16,
    binding: &[u8],
) -> LoopResult<()> {
    let id = character.as_bytes().as_slice();
    let mut tracks = vec![START];
    match uid {
        9035 => tracks.push(DIPHRAH),
        9036 => tracks.push(MAHRDIS),
        _ => {}
    }
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT track_key,value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2) AND quest_key=$3",
    )
    .bind(id)
    .bind(&tracks)
    .bind(QUEST)
    .fetch_all(&mut **tx)
    .await?;
    let mut keys = Vec::new();
    let mut before = Vec::new();
    let mut after = Vec::new();
    let mut existing = 0;
    for key in tracks {
        let track = catalogue.track(key).ok_or(DurabilityError::InvalidStoredState)?;
        let stored = rows.iter().find(|(stored, _)| stored == key);
        let current = stored.map_or(track.initial, |(_, value)| *value);
        if current == 1 || (key != START && current > 0) {
            continue;
        }
        if !(track.min..=track.max).contains(&1) {
            return refuse(LoopMoveRefusal::Rejected);
        }
        keys.push(key);
        before.push(current);
        after.push(1_i64);
        existing += usize::from(stored.is_some());
    }
    if keys.is_empty() {
        return Ok(());
    }
    let stored: Option<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT pinned_content_revision,definition_hash FROM game_character_quest_states
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE",
    )
    .bind(id)
    .bind(QUEST)
    .fetch_optional(&mut **tx)
    .await?;
    if (stored.is_none() && !capacity::new_state_fits(tx, character).await?)
        || !capacity::tracks_fit(tx, character, &keys, existing).await?
    {
        return refuse(LoopMoveRefusal::Rejected);
    }
    let hash = catalogue.definition_hash(QUEST).ok_or(DurabilityError::InvalidStoredState)?;
    let (pin, hash) = stored.unwrap_or_else(|| (catalogue.content_revision().to_owned(), hash.to_vec()));
    let original = root.revision.to_string();
    let next = root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?.to_string();
    for query in [
        "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
        "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
    ] {
        if sqlx::query(query).bind(id).bind(&next).bind(&original).execute(&mut **tx).await?.rows_affected() != 1 {
            return refuse(LoopMoveRefusal::Stale);
        }
    }
    sqlx::query(
        "INSERT INTO game_character_quest_receipts(character_id,cause_kind,cause_id,cause_ordinal,
          transition_key,quest_key,request_binding,pinned_content_revision,definition_hash,completes,
          track_keys,values_before,values_after,original_character_revision,committed_character_revision,
          level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,
          content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at)
         SELECT s.character_id,'use',encode($2,'hex')::uuid,$3::text::numeric,
          'oteryn:quest-callback/ancient-tombs-coal-basin',$4,$5,$6,$7,false,
          $8,$9,$10,$11::text::numeric,$12::text::numeric,s.level,s.level,s.total_experience,s.total_experience,
          s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,
          s.declaration_revision,s.policy_revision,s.reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint
         FROM game_character_progression_state s WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
    .bind(QUEST).bind(binding).bind(&pin).bind(&hash).bind(&keys).bind(&before).bind(&after)
    .bind(&original).bind(&next).execute(&mut **tx).await?;
    for key in keys {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,1,$4::text::numeric) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
            .bind(id).bind(key).bind(QUEST).bind(&next).execute(&mut **tx).await?;
    }
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,$5::text::numeric) ON CONFLICT(character_id,quest_key) DO UPDATE SET committed_character_revision=EXCLUDED.committed_character_revision")
        .bind(id).bind(QUEST).bind(pin).bind(hash).bind(next).execute(&mut **tx).await?;
    Ok(())
}
