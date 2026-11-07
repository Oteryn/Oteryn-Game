//! PostgreSQL encoding and history reads for the pure skull evaluator.
use super::{DurabilityError, pvp_death_request::PvpDeathContributor};
use oteryn_gameplay_combat_rules::pvp::skull::{
    FRAG_WINDOWS_US, PvpSkull, SkullHistory, SkullState, UnjustifiedPoint, evaluate_skull,
};
use sqlx::Row;

pub(super) async fn evaluate_stored_skull(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    contributor: &PvpDeathContributor,
    occurrence: &[u8; 16],
    now_us: i64,
    increment: u16,
) -> Result<SkullState, DurabilityError> {
    let row = sqlx::query(
        "SELECT skull, (extract(epoch FROM skull_until) * 1000000)::bigint AS until_us \
         FROM game_character_pvp_state WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(contributor.character_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let skull = match row.try_get::<i16, _>("skull")? {
        0 => PvpSkull::None,
        1 => PvpSkull::Red,
        2 => PvpSkull::Black,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let stored = SkullState {
        skull,
        until_us: row.try_get("until_us")?,
    };
    let lower = now_us
        .checked_sub(FRAG_WINDOWS_US[2])
        .ok_or(DurabilityError::InvalidStoredState)?;
    // PostgreSQL timestamps and the evaluator both retain all six fractional digits.
    // Exclude this occurrence: its increment is added exactly once by the rule.
    let mut history = SkullHistory::new(now_us).map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut cursor_us = lower;
    let mut cursor_id = "00000000-0000-0000-0000-000000000000".to_owned();
    loop {
        let rows = sqlx::query(
            "SELECT points_milli, death_occurrence_id::text AS occurrence, \
             (extract(epoch FROM committed_at) * 1000000)::bigint AS at_us \
             FROM game_character_unjustified_points WHERE character_id = encode($1,'hex')::uuid \
             AND death_occurrence_id <> encode($2,'hex')::uuid \
             AND committed_at > timestamptz 'epoch' + $3 * interval '1 microsecond' \
             AND (committed_at, death_occurrence_id) > \
                 (timestamptz 'epoch' + $4 * interval '1 microsecond', $5::text::uuid) \
             ORDER BY committed_at, death_occurrence_id LIMIT 256",
        )
        .bind(contributor.character_id.as_bytes().as_slice())
        .bind(occurrence.as_slice())
        .bind(lower)
        .bind(cursor_us)
        .bind(&cursor_id)
        .fetch_all(&mut **tx)
        .await?;
        for row in &rows {
            cursor_us = row.try_get("at_us")?;
            cursor_id = row.try_get("occurrence")?;
            history
                .include(UnjustifiedPoint {
                    committed_at_us: cursor_us,
                    points_milli: u16::try_from(row.try_get::<i32, _>("points_milli")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                })
                .map_err(|_| DurabilityError::InvalidStoredState)?;
        }
        if rows.len() < 256 {
            break;
        }
    }
    evaluate_skull(now_us, stored, &history, increment)
        .map_err(|_| DurabilityError::InvalidStoredState)
}
