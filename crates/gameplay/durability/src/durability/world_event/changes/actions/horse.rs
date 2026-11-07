//! Native stable deaths are escape reports, counted only from fresh durable UPDATE results.
use super::*;
use crate::durability::raid_runner::RaidStepIdentity;

/// Match the scheduler's live-World -> firing/run lock order before a native
/// stable death takes its run row. Late-target FK checks also lock the firing.
pub(crate) async fn lock_horse_projection(
    tx: &mut Transaction<'_, Postgres>,
    firing: [u8; 16],
) -> Result<(), DurabilityError> {
    let row:Option<(Vec<u8>,String)>=sqlx::query_as(
        "SELECT uuid_send(world_id),raid_key FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid",
    ).bind(firing.as_slice()).fetch_optional(&mut **tx).await?;
    if let Some((world, key)) = row
        && horse_projection(&key)
    {
        let world = WorldId::decode(&world).map_err(|_| DurabilityError::InvalidStoredState)?;
        lock(tx, world).await?;
    }
    Ok(())
}

/// Called only by record_raid_creature_deaths after its complete placed-identity validation.
/// `changed` is the number of that same transaction's state1→2 updates, never proposed deaths.
/// Replayed deaths therefore cannot decrement the shared stable count again.
pub(crate) async fn record_horse_deaths(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    identity: RaidStepIdentity,
    changed: u16,
) -> Result<(), DurabilityError> {
    if changed == 0 {
        return Ok(());
    }
    let row: Option<(Vec<u8>, String, String)> = sqlx::query_as(
        "SELECT uuid_send(world_id),reset_epoch::text,raid_key FROM game_world_raid_firings \
         WHERE firing_id=encode($1,'hex')::uuid",
    )
    .bind(identity.firing_id.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some((world, epoch, key)) = row else {
        return Err(DurabilityError::InvalidStoredState);
    };
    if stable_projection_count(&key).is_none() {
        return Ok(());
    }
    let world = WorldId::decode(&world).map_err(|_| DurabilityError::InvalidStoredState)?;
    let epoch = epoch
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    lock(tx, world).await?;
    let Some(before) = snapshot(tx, world, epoch).await? else {
        return Err(DurabilityError::Unavailable);
    };
    let now_ms: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    if now_ms < before.updated_at_ms {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut after = before.state.clone();
    after.advance(now_ms);
    // A simultaneous NPC escape may already have replaced this cohort. Its old
    // native actors retire through LOST rather than debiting the replacement.
    if !projection_matches(&key, &after.horse, now_ms) {
        return Ok(());
    }
    for _ in 0..changed {
        if !after.horse.escape(now_ms) {
            break;
        }
    }
    after.advance(now_ms);
    if after != before.state {
        write(
            tx,
            world,
            epoch,
            &before,
            &after,
            now_ms,
            Proof::Tick {
                node,
                channel: identity.channel,
                generation: identity.generation,
            },
        )
        .await?;
    }
    Ok(())
}
