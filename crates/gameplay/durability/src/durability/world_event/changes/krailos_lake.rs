//! Lock the World lake in the same transaction as the quest receipt and item use.
use crate::{durability::DurabilityError, foundation::WorldId};
use oteryn_gameplay_quests::quest::krailos_lake::{Action, Lake};
use sqlx::{Postgres, Transaction};

pub(crate) async fn advance(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    expected_epoch: Option<u64>,
    action: Action,
) -> Result<bool, DurabilityError> {
    let world = world.as_bytes().as_slice();
    let epoch: Option<String> = sqlx::query_scalar(
        "SELECT reset_epoch::text FROM game_world_reset_active
         WHERE world_id=encode($1,'hex')::uuid FOR SHARE",
    )
    .bind(world)
    .fetch_optional(&mut **tx)
    .await?;
    let epoch = epoch
        .as_deref()
        .unwrap_or("0")
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if expected_epoch.is_some_and(|expected| expected != epoch) {
        return Ok(false);
    }

    sqlx::query(
        "INSERT INTO game_krailos_lake VALUES (encode($1,'hex')::uuid,$2::text::numeric,0)
         ON CONFLICT DO NOTHING",
    )
    .bind(world)
    .bind(epoch.to_string())
    .execute(&mut **tx)
    .await?;
    let (previous_epoch, previous_progress): (String, i16) = sqlx::query_as(
        "SELECT reset_epoch::text, progress FROM game_krailos_lake
         WHERE world_id=encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(world)
    .fetch_one(&mut **tx)
    .await?;
    let previous_epoch = previous_epoch
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut lake = Lake::from_stored(previous_epoch, previous_progress)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if !lake.advance(epoch, action) {
        return Ok(false);
    }
    let (epoch, progress) = lake.stored();
    if (epoch, progress) != (previous_epoch, previous_progress) {
        sqlx::query(
            "UPDATE game_krailos_lake SET reset_epoch=$2::text::numeric, progress=$3
             WHERE world_id=encode($1,'hex')::uuid",
        )
        .bind(world)
        .bind(epoch.to_string())
        .bind(progress)
        .execute(&mut **tx)
        .await?;
    }
    Ok(true)
}
