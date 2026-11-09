//! Current occurrence-clock read at the Channel's physical map publication boundary.
use super::{DurabilityError, NodeIncarnationProof, RaidStepIdentity, effects};
use crate::foundation::WorldId;
use sqlx::{Postgres, Row, Transaction};

/// Retain this transaction and the physical Item/Channel owner through publication.
/// Pending announcements do not gate scenery. Finished runs retain their source
/// scenery until reset; LOST and mismatched targets never authorize publication.
pub async fn feroxa_map_elapsed_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    identity: RaidStepIdentity,
    world: WorldId,
    reset_epoch: u64,
    bundle_digest: [u8; 32],
    now_ms: i64,
) -> Result<u64, DurabilityError> {
    if reset_epoch == 0 || bundle_digest == [0; 32] || now_ms < 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if !effects::world_current(tx, identity.firing_id).await?
        || !effects::current(
            tx,
            node,
            identity.firing_id,
            identity.channel,
            identity.generation,
        )
        .await?
    {
        return Err(DurabilityError::Unavailable);
    }
    let firing = sqlx::query("SELECT uuid_send(world_id) AS world_id,reset_epoch::text,bundle_digest,raid_key,state,fired_at_ms FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(identity.firing_id.as_slice()).fetch_optional(&mut **tx).await?
        .ok_or(DurabilityError::Unavailable)?;
    if firing.try_get::<Vec<u8>, _>("world_id")?.as_slice() != world.as_bytes().as_slice()
        || firing.try_get::<String, _>("reset_epoch")? != reset_epoch.to_string()
        || firing.try_get::<Vec<u8>, _>("bundle_digest")?.as_slice() != bundle_digest.as_slice()
        || firing.try_get::<String, _>("raid_key")? != super::super::raid_schedule::FEROXA_KEY
        || !matches!(firing.try_get::<i16, _>("state")?, 1 | 2)
    {
        return Err(DurabilityError::Unavailable);
    }
    let target: Option<String> = sqlx::query_scalar("SELECT owner_generation::text FROM game_world_raid_targets WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid FOR SHARE")
        .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    if target.as_deref() != Some(&identity.generation.get().to_string()) {
        return Err(DurabilityError::Unavailable);
    }
    let run = sqlx::query("SELECT owner_generation::text,state FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    if let Some(run) = run {
        if run.try_get::<String, _>("owner_generation")? != identity.generation.get().to_string()
            || !matches!(run.try_get::<i16, _>("state")?, 1 | 2)
        {
            return Err(DurabilityError::Unavailable);
        }
    } else if firing.try_get::<i16, _>("state")? != 1 {
        return Err(DurabilityError::Unavailable);
    }
    // A newly selected firing may not have committed its first announcement yet.
    // The firing lock also fences insertion of that initial Channel run.
    now_ms
        .checked_sub(firing.try_get::<i64, _>("fired_at_ms")?)
        .and_then(|elapsed| u64::try_from(elapsed).ok())
        .ok_or(DurabilityError::InvalidStoredState)
}
