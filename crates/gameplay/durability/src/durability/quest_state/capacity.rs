//! Shared quest storage admission under the already-held Character root lock.
//! A writer must leave enough space for the existing bounded runtime projection.

use super::{
    CharacterId, DurabilityError, QUESTSTATE0_RL_01, QUESTSTATE0_RL_05, QUESTSTATE0_RL_08_STATES,
    QUESTSTATE0_RL_08_TRACKS,
};
use oteryn_gameplay_quests::quest::monk_shrines::{COUNT_TRACK, FLAG_PREFIX};
use sqlx::PgConnection;
use crate::durability::character_hazard::PRIMAL_MENACE_KILLED_TRACK;

/// Called only when the requested quest state does not already exist.
pub(in crate::durability) async fn new_state_fits(
    tx: &mut PgConnection,
    character: CharacterId,
) -> Result<bool, DurabilityError> {
    states_fit(tx, character, 1).await
}

pub(in crate::durability) async fn states_fit(
    tx: &mut PgConnection,
    character: CharacterId,
    additional: usize,
) -> Result<bool, DurabilityError> {
    let states: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_states \
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(tx)
    .await?;
    Ok(usize::try_from(states)
        .unwrap_or(usize::MAX)
        .saturating_add(additional)
        <= QUESTSTATE0_RL_05.min(QUESTSTATE0_RL_08_STATES))
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
    let (tracks, has_counter, has_flags, has_primal): (i64, bool, bool, bool) = sqlx::query_as(
        "SELECT count(*), coalesce(bool_or(track_key=$2),false), \
           coalesce(bool_or(left(track_key,length($3))=$3),false), coalesce(bool_or(track_key=$4),false) \
         FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .bind(COUNT_TRACK)
    .bind(FLAG_PREFIX)
    .bind(PRIMAL_MENACE_KILLED_TRACK)
    .fetch_one(tx)
    .await?;
    // A flag-only Character needs one additional counter in its read projection.
    // This is the existing monk-shrine reservation, shared by every quest writer.
    let has_counter = has_counter || keys.contains(&COUNT_TRACK);
    let has_flags = has_flags || keys.iter().any(|key| key.starts_with(FLAG_PREFIX));
    // Primal death receipts derive a track without going through this writer.
    // Reserve it before the first death, unless it is already stored or written.
    let has_primal = has_primal || keys.contains(&PRIMAL_MENACE_KILLED_TRACK);
    let after = usize::try_from(tracks)
        .unwrap_or(usize::MAX)
        .saturating_add(additional)
        .saturating_add(usize::from(has_flags && !has_counter))
        .saturating_add(usize::from(!has_primal));
    Ok(after <= QUESTSTATE0_RL_01.min(QUESTSTATE0_RL_08_TRACKS))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Connection;

    #[test]
    fn audit_capacity_reserves_primal_and_monk_projection_for_inserts_and_updates()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
            return Ok(());
        };
        std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async move {
        let mut connection = sqlx::PgConnection::connect(&url).await?;
        // Isolate the admission query's inputs without bypassing a production
        // writer's receipt guards or mutating retained Character data.
        sqlx::query("CREATE TEMP TABLE game_character_quest_tracks(character_id uuid,track_key text,value bigint)")
            .execute(&mut connection).await?;
        let character = CharacterId::from_bytes([1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1])?;
        sqlx::query("INSERT INTO game_character_quest_tracks SELECT encode($1,'hex')::uuid,'oteryn:track/'||n,0 FROM generate_series(1,$2) n")
            .bind(character.as_bytes().as_slice()).bind((QUESTSTATE0_RL_08_TRACKS - 1) as i64)
            .execute(&mut connection).await?;
        assert!(!tracks_fit(&mut connection, character, &["oteryn:track/new"], 0).await?);
        let primal = crate::durability::character_hazard::PRIMAL_MENACE_KILLED_TRACK;
        assert!(tracks_fit(&mut connection, character, &[primal], 0).await?);
        // Updating a full stored collection must still account for the derived flag.
        sqlx::query("INSERT INTO game_character_quest_tracks VALUES(encode($1,'hex')::uuid,'oteryn:track/new',0)")
            .bind(character.as_bytes().as_slice()).execute(&mut connection).await?;
        assert!(!tracks_fit(&mut connection, character, &["oteryn:track/new"], 1).await?);
        sqlx::query("UPDATE game_character_quest_tracks SET track_key=$2 WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:track/new'")
            .bind(character.as_bytes().as_slice()).bind(primal).execute(&mut connection).await?;
        assert!(tracks_fit(&mut connection, character, &[primal], 1).await?);
        // Monk's virtual counter and Primal's virtual flag each occupy a slot.
        sqlx::query("DELETE FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(primal).execute(&mut connection).await?;
        let flag = format!("{FLAG_PREFIX}guidance");
        assert!(!tracks_fit(&mut connection, character, &[&flag], 0).await?);
        sqlx::query("DELETE FROM game_character_quest_tracks WHERE track_key IN ('oteryn:track/1','oteryn:track/2')")
            .execute(&mut connection).await?;
        assert!(tracks_fit(&mut connection, character, &[&flag], 0).await?);
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            })
        })?.join().map_err(|_| "capacity test thread panicked")?
    }
}
