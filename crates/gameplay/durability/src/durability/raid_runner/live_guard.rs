//! Independently current read at the physical spawn/publication boundary.
//! The caller retains this transaction through the live effect; the returned
//! boolean is not an ownership token and must never escape that owner turn.
use super::{
    CommittedRaidAction, CommittedRaidStep, DurabilityError, NodeIncarnationProof, effects,
};
use crate::foundation::WorldId;
use sqlx::{Postgres, Row, Transaction};

/// Lock the same reset, assignment, firing, run and effect rows as the durable
/// writer before applying a pending announcement/wave to current live owners.
/// False means the identical effect is already acknowledged and must not be
/// published or spawned again. A changed pin, body, generation or lost effect
/// authorizes no live action. Existing lock order is reset -> assignment ->
/// firing -> run -> effect; the caller's physical Item guard stays held too.
pub async fn raid_effect_pending_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    step: &CommittedRaidStep,
    world: WorldId,
    reset_epoch: u64,
    bundle_digest: [u8; 32],
) -> Result<bool, DurabilityError> {
    let id = step.identity;
    if reset_epoch == 0
        || bundle_digest == [0; 32]
        || !matches!(
            step.action,
            CommittedRaidAction::Announcement { .. } | CommittedRaidAction::Wave { .. }
        )
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    if !effects::world_current(tx, id.firing_id).await?
        || !effects::current(tx, node, id.firing_id, id.channel, id.generation).await?
    {
        return Err(DurabilityError::Unavailable);
    }
    let firing = sqlx::query("SELECT uuid_send(world_id) AS world_id,reset_epoch::text,bundle_digest,state FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(id.firing_id.as_slice()).fetch_optional(&mut **tx).await?
        .ok_or(DurabilityError::Unavailable)?;
    if firing.try_get::<Vec<u8>, _>("world_id")?.as_slice() != world.as_bytes().as_slice()
        || firing.try_get::<String, _>("reset_epoch")? != reset_epoch.to_string()
        || firing.try_get::<Vec<u8>, _>("bundle_digest")?.as_slice() != bundle_digest.as_slice()
    {
        return Err(DurabilityError::Unavailable);
    }
    let run = sqlx::query("SELECT owner_generation::text,state FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?.ok_or(DurabilityError::Unavailable)?;
    let owner = id.generation.get().to_string();
    if run.try_get::<String, _>("owner_generation")? != owner {
        return Err(DurabilityError::Unavailable);
    }
    let row = sqlx::query("SELECT *,owner_generation::text AS retained_owner FROM game_world_raid_effects WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND step_ordinal=$3 FOR UPDATE")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
        .bind(i32::try_from(id.ordinal).map_err(|_| DurabilityError::InvalidStoredState)?)
        .fetch_optional(&mut **tx).await?.ok_or(DurabilityError::Unavailable)?;
    // Numeric owner_generation is decoded explicitly to text. Avoid trusting
    // the proposed body's replay bit; equality concerns the committed action.
    let effect_owner: String = row.try_get("retained_owner")?;
    let stored = effects::decode(tx, id, &row).await?;
    if effect_owner != owner || stored.identity != id || stored.action != step.action {
        return Err(DurabilityError::Unavailable);
    }
    match row.try_get::<i16, _>("state")? {
        2 => Ok(false),
        1 if firing.try_get::<i16, _>("state")? == 1 && run.try_get::<i16, _>("state")? == 1 => {
            Ok(true)
        }
        _ => Err(DurabilityError::Unavailable),
    }
}
