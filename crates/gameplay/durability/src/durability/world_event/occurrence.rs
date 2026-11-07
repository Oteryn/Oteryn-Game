//! Immutable occurrence decisions and bounded definition registration.
use super::*;
use oteryn_gameplay_world_change::{
    decision::NativeDecision,
    rules::{Catalogue, Definition},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_STATES: i64 = 512;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    schema: u8,
    world: [u8; 16],
    epoch: u64,
    month: u8,
    day: u8,
    native: NativeDecision,
    definitions: Vec<Definition>,
}

pub(super) struct Bound {
    pub native: oteryn_gameplay_world_change::daily::DailyWorldChanges,
    pub month: u8,
    pub day: u8,
    pub catalogue: Catalogue,
}

pub(super) async fn lock(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn.world-change.binding.v1:'||encode($1,'hex'),0))")
        .bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Bound, DurabilityError> {
    let (bytes,digest,month,day): (Vec<u8>,Vec<u8>,i16,i16) = sqlx::query_as("SELECT o.decision,o.decision_digest,c.month,c.day FROM game_world_change_occurrences o JOIN game_world_change_calendar c USING(world_id,reset_epoch) WHERE o.world_id=encode($1,'hex')::uuid AND o.reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
    if bytes.len() > 262144 || Sha256::digest(&bytes).as_slice() != digest {
        return Err(DurabilityError::InvalidStoredState);
    }
    let stored: Stored =
        serde_json::from_slice(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
    let native = stored
        .native
        .restore()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if stored.schema != 1
        || stored.world != *world.as_bytes()
        || stored.epoch != epoch
        || native.reset_epoch != epoch
        || !(1..=12).contains(&stored.month)
        || !(1..=31).contains(&stored.day)
        || i16::from(stored.month) != month
        || i16::from(stored.day) != day
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let catalogue = Catalogue::from_definitions(stored.definitions)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Bound {
        native,
        month: stored.month,
        day: stored.day,
        catalogue,
    })
}

pub(super) async fn capture(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    month: u8,
    day: u8,
    catalogue: &Catalogue,
) -> Result<Bound, DurabilityError> {
    lock(tx, world).await?;
    let native =
        oteryn_gameplay_world_change::daily::roll_calendar(world.as_bytes(), epoch, month, day);
    let bytes = serde_json::to_vec(&Stored {
        schema: 1,
        world: *world.as_bytes(),
        epoch,
        month,
        day,
        native: NativeDecision::capture(&native),
        definitions: catalogue.definitions().to_vec(),
    })
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    if bytes.len() > 262144 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    sqlx::query("INSERT INTO game_world_change_occurrences(world_id,reset_epoch,decision,decision_digest) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&bytes).bind(digest.as_slice()).execute(&mut **tx).await?;
    let bound = load(tx, world, epoch).await?;
    let keys: Vec<_> = bound
        .catalogue
        .definitions()
        .iter()
        .map(|d| d.key.as_str())
        .collect();
    let (total,missing): (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_world_change_states WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric),(SELECT count(*) FROM unnest($3::text[]) AS k(change_key) WHERE NOT EXISTS(SELECT 1 FROM game_world_change_states s WHERE s.world_id=encode($1,'hex')::uuid AND s.reset_epoch=$2::text::numeric AND s.change_key=k.change_key))")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&keys).fetch_one(&mut **tx).await?;
    if total > MAX_STATES - missing {
        return Err(DurabilityError::InvalidStoredState);
    }
    for definition in bound.catalogue.definitions() {
        insert(tx, world, epoch, &bound, definition).await?;
    }
    Ok(bound)
}

pub(super) async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    bound: &Bound,
    definition: &Definition,
) -> Result<(), DurabilityError> {
    definition
        .validate()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let bytes = serde_json::to_vec(definition).map_err(|_| DurabilityError::InvalidStoredState)?;
    if bytes.len() > 16384 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let selected = definition.selected(
        world.as_bytes(),
        epoch,
        bound.month,
        bound.day,
        &bound.native,
    );
    let counter = definition.counter.as_ref().filter(|_| selected);
    let active = counter.map_or(selected, |c| c.initial_active);
    let creature = counter.map(|c| c.creature.as_str());
    let goal = counter.map(|c| i64::from(c.kills));
    let goal_active = counter.is_none_or(|c| c.active_on_goal);
    sqlx::query("INSERT INTO game_world_change_states(world_id,reset_epoch,change_key,active,creature_key,kill_goal,goal_active,definition) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&definition.key).bind(active).bind(creature).bind(goal).bind(goal_active).bind(&bytes).execute(&mut **tx).await?;
    let (stored,matches): (Option<Vec<u8>>,bool) = sqlx::query_as("SELECT definition,(creature_key IS NOT DISTINCT FROM $4::text) AND (kill_goal IS NOT DISTINCT FROM $5::bigint) AND goal_active=$6 FROM game_world_change_states WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND change_key=$3 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&definition.key).bind(creature).bind(goal).bind(goal_active).fetch_one(&mut **tx).await?;
    if !matches || stored.as_ref().is_some_and(|stored| stored != &bytes) {
        return Err(DurabilityError::InvalidStoredState);
    }
    if stored.is_none() {
        sqlx::query("UPDATE game_world_change_states SET definition=$4 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND change_key=$3 AND definition IS NULL")
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&definition.key).bind(bytes).execute(&mut **tx).await?;
    }
    Ok(())
}

pub(super) async fn capacity(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    key: &str,
) -> Result<(), DurabilityError> {
    let full: bool = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_world_change_states WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric)>=512 AND NOT EXISTS(SELECT 1 FROM game_world_change_states WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND change_key=$3)")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(key).fetch_one(&mut **tx).await?;
    if full {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
