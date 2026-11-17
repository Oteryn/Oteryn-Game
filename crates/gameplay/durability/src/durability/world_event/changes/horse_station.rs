use super::*;
use oteryn_gameplay_world_change::horse_station::HorseStation;

/// Advance the shared phase using the immutable activation timestamp. The
/// reset transaction serializes this update with other World changes.
pub(super) async fn at_save(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    timestamp: i64,
) -> Result<(), DurabilityError> {
    let deadline: Option<i64> = sqlx::query_scalar(
        "INSERT INTO game_horse_stations(world_id) VALUES(encode($1,'hex')::uuid)
         ON CONFLICT(world_id) DO UPDATE SET world_id=EXCLUDED.world_id
         RETURNING loose_until_ms",
    )
    .bind(world.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if let Some(deadline) = deadline
        && HorseStation::LooseUntil(deadline).at_save(timestamp) == HorseStation::Confined
    {
        sqlx::query(
            "UPDATE game_horse_stations SET loose_until_ms=NULL WHERE world_id=encode($1,'hex')::uuid",
        )
        .bind(world.as_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
