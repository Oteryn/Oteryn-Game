//! Shared quest storage admission under the already-held Character root lock.
//! A writer must leave enough space for the existing bounded runtime projection.

use super::{
    CharacterId, DurabilityError, QUESTSTATE0_RL_01, QUESTSTATE0_RL_05, QUESTSTATE0_RL_08_STATES,
    QUESTSTATE0_RL_08_TRACKS,
};
use oteryn_gameplay_quests::quest::monk_shrines::{COUNT_TRACK, FLAG_PREFIX};
use sqlx::PgConnection;

/// Called only when the requested quest state does not already exist.
pub(in crate::durability) async fn new_state_fits(
    tx: &mut PgConnection,
    character: CharacterId,
) -> Result<bool, DurabilityError> {
    let states: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_states \
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(tx)
    .await?;
    Ok(usize::try_from(states).unwrap_or(usize::MAX)
        < QUESTSTATE0_RL_05.min(QUESTSTATE0_RL_08_STATES))
}

/// `keys` are distinct tracks this writer may insert; `existing` counts those
/// already present. Updating existing tracks consumes no additional capacity.
pub(in crate::durability) async fn tracks_fit(
    tx: &mut PgConnection,
    character: CharacterId,
    keys: &[&str],
    existing: usize,
) -> Result<bool, DurabilityError> {
    let additional = keys
        .len()
        .checked_sub(existing)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if additional == 0 {
        return Ok(true);
    }
    let (tracks, has_counter, has_flags): (i64, bool, bool) = sqlx::query_as(
        "SELECT count(*), coalesce(bool_or(track_key=$2),false), \
           coalesce(bool_or(left(track_key,length($3))=$3),false) \
         FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .bind(COUNT_TRACK)
    .bind(FLAG_PREFIX)
    .fetch_one(tx)
    .await?;
    // A flag-only Character needs one additional counter in its read projection.
    // This is the existing monk-shrine reservation, shared by every quest writer.
    let has_counter = has_counter || keys.contains(&COUNT_TRACK);
    let has_flags = has_flags || keys.iter().any(|key| key.starts_with(FLAG_PREFIX));
    let after = usize::try_from(tracks)
        .unwrap_or(usize::MAX)
        .saturating_add(additional)
        .saturating_add(usize::from(has_flags && !has_counter));
    Ok(after <= QUESTSTATE0_RL_01.min(QUESTSTATE0_RL_08_TRACKS))
}
