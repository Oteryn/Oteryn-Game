//! The reset/day transaction is the sole writer of native binding decisions.
use super::*;
use oteryn_gameplay_world_change::bindings::NativeBindings;

fn encoded(day: &DailyWorldChanges) -> Result<Vec<u8>, DurabilityError> {
    let bytes = NativeBindings::compile(day)
        .and_then(|binding| binding.encoded())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if bytes.is_empty() || bytes.len() > 65536 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(bytes)
}

pub(super) async fn capture(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    day: &DailyWorldChanges,
) -> Result<(), DurabilityError> {
    let bytes = encoded(day)?;
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    sqlx::query("INSERT INTO game_world_change_native_bindings(world_id,reset_epoch,bindings,bindings_digest) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(day.reset_epoch.to_string())
        .bind(bytes).bind(digest.as_slice()).execute(&mut **tx).await?;
    verify(tx, world, day, true).await
}

pub(super) async fn verify(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    day: &DailyWorldChanges,
    required: bool,
) -> Result<(), DurabilityError> {
    let expected = encoded(day)?;
    let stored: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT bindings,bindings_digest FROM game_world_change_native_bindings WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(day.reset_epoch.to_string())
        .fetch_optional(&mut **tx).await?;
    if let Some((bytes, digest)) = stored {
        if bytes != expected || Sha256::digest(&bytes).as_slice() != digest {
            return Err(DurabilityError::InvalidStoredState);
        }
    } else if required {
        return Err(DurabilityError::InvalidStoredState);
    } else {
        // Only legacy-writer decisions can take this one-time upgrade path.
        let digest: [u8; 32] = Sha256::digest(&expected).into();
        sqlx::query("INSERT INTO game_world_change_native_bindings(world_id,reset_epoch,bindings,bindings_digest) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4) ON CONFLICT DO NOTHING")
            .bind(world.as_bytes().as_slice()).bind(day.reset_epoch.to_string())
            .bind(&expected).bind(digest.as_slice()).execute(&mut **tx).await?;
        let (bytes, digest): (Vec<u8>, Vec<u8>) = sqlx::query_as("SELECT bindings,bindings_digest FROM game_world_change_native_bindings WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
            .bind(world.as_bytes().as_slice()).bind(day.reset_epoch.to_string()).fetch_one(&mut **tx).await?;
        if bytes != expected || Sha256::digest(&bytes).as_slice() != digest {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    if !required {
        sqlx::query("UPDATE game_world_change_days SET native_binding_required=true WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND NOT native_binding_required")
            .bind(world.as_bytes().as_slice()).bind(day.reset_epoch.to_string()).execute(&mut **tx).await?;
    }
    Ok(())
}
