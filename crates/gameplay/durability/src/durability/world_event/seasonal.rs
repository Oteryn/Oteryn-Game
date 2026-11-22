//! Immutable production and active-epoch reads for seasonal consumers.
use super::*;
use oteryn_gameplay_world_change::seasonal::{KEYS, Occurrence, at_save};

pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<(), DurabilityError> {
    let (year, month, day): (i32, i16, i16) = sqlx::query_as(
        "SELECT extract(year FROM day)::integer,extract(month FROM day)::smallint,
         extract(day FROM day)::smallint FROM (
             SELECT activated_at AT TIME ZONE 'Europe/Berlin' AS day
             FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid
             AND reset_epoch=$2::text::numeric
         ) AS save",
    )
    .bind(world.as_bytes().as_slice())
    .bind(epoch.to_string())
    .fetch_one(&mut **tx)
    .await?;
    let month = u8::try_from(month).map_err(|_| DurabilityError::InvalidStoredState)?;
    let day = u8::try_from(day).map_err(|_| DurabilityError::InvalidStoredState)?;
    for event in at_save(world.as_bytes(), epoch, year, month, day) {
        sqlx::query(
            "INSERT INTO game_world_seasonal_occurrences(world_id,reset_epoch,event_key,occurrence_id)
             VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4)",
        )
        .bind(world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .bind(event.key)
        .bind(event.id.as_slice())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub(super) async fn active(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    key: &str,
    epoch: Option<u64>,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_world_seasonal_occurrences s
         JOIN game_world_reset_active a USING(world_id,reset_epoch)
         WHERE s.world_id=encode($1,'hex')::uuid AND s.event_key=$2
         AND ($3::text IS NULL OR s.reset_epoch=$3::text::numeric))",
    )
    .bind(world.as_bytes().as_slice())
    .bind(key)
    .bind(epoch.map(|epoch| epoch.to_string()))
    .fetch_one(&mut **tx)
    .await?)
}

impl DurabilityRoot {
    /// One committed World snapshot for map layers and NPC placement.
    pub async fn active_world_seasonal_occurrences(
        &self,
        world: WorldId,
        epoch: u64,
    ) -> Result<Vec<Occurrence>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            lock_admission_relations(&mut tx).await?;
            let current: Option<String> = sqlx::query_scalar(
                "SELECT reset_epoch::text FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid",
            )
            .bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let current = current.map_or(Ok(0), |value| value.parse::<u64>())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if epoch != current {
                return Err(DurabilityError::InvalidStoredState);
            }
            let rows: Vec<(String, Vec<u8>)> = sqlx::query_as(
                "SELECT event_key,occurrence_id FROM game_world_seasonal_occurrences
                 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric
                 ORDER BY event_key LIMIT 4",
            )
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_all(&mut *tx).await?;
            let occurrences = rows.into_iter().map(|(key, id)| {
                if !KEYS.contains(&key.as_str()) {
                    return Err(DurabilityError::InvalidStoredState);
                }
                Ok(Occurrence {
                    key,
                    id: id.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,
                })
            }).collect::<Result<_, _>>()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(occurrences)
        })).await
    }
}
