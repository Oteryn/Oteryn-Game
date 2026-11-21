//! Crystal 9f5a72c grimvale.lua:8-23 and functions/game.lua:117-118.
//! Caller holds the current fenced World/reset and raid scheduler locks.
use super::WorldId;
use crate::durability::DurabilityError;
use oteryn_gameplay_encounter_rules::raid::RaidRandom;
use sqlx::{Postgres, Transaction};

pub(super) async fn selected(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    reset_epoch: u64,
    ordinal: u64,
    now_ms: i64,
    prior_fired_at: Option<i64>,
) -> Result<bool, DurabilityError> {
    // Existing firings also establish Active when upgrading an admitted epoch.
    sqlx::query("INSERT INTO game_world_feroxa_occurrences(world_id,reset_epoch,phase,selected_at_ms) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(reset_epoch.to_string())
        .bind(if prior_fired_at.is_some() { 1_i16 } else { 0_i16 }).bind(prior_fired_at)
        .execute(&mut **tx).await?;
    let (chance, phase): (i16, i16) = sqlx::query_as("SELECT chance,phase FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(reset_epoch.to_string()).fetch_one(&mut **tx).await?;
    if phase >= 1 {
        return Ok(false);
    }
    if chance <= 5 {
        sqlx::query("UPDATE game_world_feroxa_occurrences SET phase=1,selected_at_ms=$3 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
            .bind(world.as_bytes().as_slice()).bind(reset_epoch.to_string()).bind(now_ms)
            .execute(&mut **tx).await?;
        return Ok(true);
    }
    // Retrying an uncommitted check reuses its World/epoch/ordinal draw. The
    // replacement can select only on the next source minute, never this check.
    let mut seed = [0; 32];
    seed[..16].copy_from_slice(world.as_bytes());
    seed[16..24].copy_from_slice(&reset_epoch.to_be_bytes());
    seed[24..].copy_from_slice(b"FeroxaV1");
    let chance = RaidRandom::new(seed, *world.as_bytes())
        .feroxa_chance(ordinal)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("UPDATE game_world_feroxa_occurrences SET chance=$3 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(reset_epoch.to_string()).bind(i16::from(chance))
        .execute(&mut **tx).await?;
    Ok(false)
}
