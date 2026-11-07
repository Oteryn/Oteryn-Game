//! Content-bound event members share the existing raid scheduler and effect owner.
use super::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EventRaidRule {
    pub key: String,
    pub member: u16,
    pub condition: String,
}

pub(crate) fn raid_rule(key: &str) -> Result<Option<(EventKind, EventRaidRule)>, DurabilityError> {
    let catalogue: serde_json::Value =
        serde_json::from_str(CATALOGUE_JSON).map_err(|_| DurabilityError::InvalidStoredState)?;
    let entries = catalogue["entries"]
        .as_array()
        .filter(|e| e.len() <= 256)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut found = None;
    for entry in entries {
        let Some(kind) = entry["key"].as_str().and_then(EventKind::from_key) else {
            continue;
        };
        // Reserve declared event raids even before publication. Without an
        // admitted timed state the scheduler refuses them, rather than falling
        // through to an unconditional calendar firing.
        let Some(rules) = entry["binding"]["raids"].as_array() else {
            continue;
        };
        if rules.len() > 16 {
            return Err(DurabilityError::InvalidStoredState);
        }
        for raw in rules {
            let rule: EventRaidRule = serde_json::from_value(raw.clone())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if rule.member >= 128 || rule.key.len() > 160 || rule.condition != "hatch" {
                return Err(DurabilityError::InvalidStoredState);
            }
            if rule.key == key {
                if found.is_some() {
                    return Err(DurabilityError::InvalidStoredState);
                }
                found = Some((kind, rule));
            }
        }
    }
    Ok(found)
}

pub(crate) fn admits(state: &EventState, rule: &EventRaidRule, now: i64) -> bool {
    matches!(
        state.phase,
        oteryn_gameplay_world_change::timed::Phase::Active
            | oteryn_gameplay_world_change::timed::Phase::Victory
    ) && state.object_ready(rule.member, now)
        && state
            .lifecycle
            .objects
            .get(&rule.member)
            .is_some_and(|object| {
                state.lifecycle.firing_versions.get(&rule.member) != Some(&object.revision())
            })
}

pub(crate) async fn record_firing(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    kind: EventKind,
    rule: &EventRaidRule,
    state: &mut EventState,
    firing: [u8; 16],
) -> Result<(), DurabilityError> {
    let origin = RaidOrigin::from_state(kind, epoch, rule.member, state)?;
    seal_raid_origin(tx, firing, &origin).await?;
    state
        .lifecycle
        .firing_versions
        .insert(rule.member, origin.version);
    state.lifecycle.last_firing.insert(rule.member, firing);
    save(tx, world, kind, state, origin.cycle).await
}

/// Acquire the cycle before any firing row, matching the scheduler's lock order.
pub(crate) async fn lock_origin(
    tx: &mut Transaction<'_, Postgres>,
    firing: [u8; 16],
) -> Result<Option<(WorldId, RaidOrigin)>, DurabilityError> {
    let Some(origin) = raid_origin(tx, firing).await? else {
        return Ok(None);
    };
    let raw: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(world_id) FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid")
        .bind(firing.as_slice()).fetch_one(&mut **tx).await?;
    let world = WorldId::decode(&raw).map_err(|_| DurabilityError::InvalidStoredState)?;
    lock_cycle(tx, world, origin.kind, origin.cycle).await?;
    Ok(Some((world, origin)))
}

pub(crate) async fn acknowledge_hatch(
    tx: &mut Transaction<'_, Postgres>,
    locked: Option<&(WorldId, RaidOrigin)>,
    placed: bool,
    firing: [u8; 16],
    now: i64,
) -> Result<(), DurabilityError> {
    let Some((world, origin)) = locked else {
        return Ok(());
    };
    if origin.kind != EventKind::SpringIntoLife {
        return Ok(());
    }
    let raw: Option<serde_json::Value> = sqlx::query_scalar("SELECT state FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(origin.kind.key()).bind(origin.cycle).fetch_optional(&mut **tx).await?;
    let Some(raw) = raw else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let mut state: EventState =
        serde_json::from_value(raw).map_err(|_| DurabilityError::InvalidStoredState)?;
    if now >= state.end_ms
        || now < state.observed_ms
        || state.lifecycle.generation != origin.generation
    {
        return Ok(());
    }
    if !placed {
        // One nest actor is planned per target. Wait for every Channel's
        // placement report before retrying an entirely failed physical hatch.
        let all_failed: bool = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_world_raid_creature_placements WHERE firing_id=encode($1,'hex')::uuid)=(SELECT count(*) FROM game_world_raid_targets WHERE firing_id=encode($1,'hex')::uuid) AND NOT EXISTS(SELECT 1 FROM game_world_raid_creature_placements WHERE firing_id=encode($1,'hex')::uuid AND placed)")
            .bind(firing.as_slice()).fetch_one(&mut **tx).await?;
        if all_failed
            && state
                .lifecycle
                .objects
                .get(&origin.member)
                .is_some_and(|o| o.revision() == origin.version)
        {
            let object = state
                .lifecycle
                .objects
                .get_mut(&origin.member)
                .ok_or(DurabilityError::InvalidStoredState)?;
            object
                .reset()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            object
                .begin(now, oteryn_gameplay_world_change::timed::HOUR)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            save(tx, *world, origin.kind, &state, origin.cycle).await?;
        }
        return Ok(());
    }
    if state
        .lifecycle
        .complete_hatch(
            origin.member,
            origin.version,
            now,
            oteryn_gameplay_world_change::timed::HOUR,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?
    {
        state
            .advance(origin.kind, now, Input::Tick, 0)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        save(tx, *world, origin.kind, &state, origin.cycle).await?;
    }
    Ok(())
}

pub(crate) async fn origin_current(
    tx: &mut Transaction<'_, Postgres>,
    locked: Option<&(WorldId, RaidOrigin)>,
) -> Result<bool, DurabilityError> {
    let Some((world, origin)) = locked else {
        return Ok(true);
    };
    if origin.kind != EventKind::SpringIntoLife {
        return Ok(true);
    }
    let raw: Option<serde_json::Value> = sqlx::query_scalar("SELECT state FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 AND (state->>'end_ms')::bigint > (extract(epoch FROM statement_timestamp())*1000)::bigint")
        .bind(world.as_bytes().as_slice()).bind(origin.kind.key()).bind(origin.cycle).fetch_optional(&mut **tx).await?;
    let Some(raw) = raw else {
        return Ok(false);
    };
    let state: EventState =
        serde_json::from_value(raw).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(matches!(
        state.phase,
        oteryn_gameplay_world_change::timed::Phase::Active
            | oteryn_gameplay_world_change::timed::Phase::Victory
    ) && state.lifecycle.generation == origin.generation
        && state.lifecycle.firing_versions.get(&origin.member) == Some(&origin.version))
}

/// Persistent actors from superseded nest firings leave through LOST cleanup.
pub(crate) async fn retire_stale(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    now: i64,
) -> Result<(), DurabilityError> {
    sqlx::query("WITH stale AS (SELECT f.firing_id FROM game_world_raid_firings f JOIN game_world_timed_raid_origins o USING(firing_id) LEFT JOIN game_world_timed_events e ON e.world_id=f.world_id AND e.event_key='spring_into_life' AND e.cycle_id=(o.origin->>'cycle')::bigint WHERE f.world_id=encode($1,'hex')::uuid AND f.state IN(1,2) AND NOT EXISTS(SELECT 1 FROM game_world_raid_retirements r WHERE r.firing_id=f.firing_id) AND o.origin->>'kind'='spring_into_life' AND (e.state IS NULL OR e.state->>'phase'='Failure' OR (e.state->>'end_ms')::bigint <= $2 OR e.state->'lifecycle'->'generation' IS DISTINCT FROM o.origin->'generation' OR e.state->'lifecycle'->'firing_versions'->(o.origin->>'member') IS DISTINCT FROM o.origin->'version') ORDER BY f.firing_id LIMIT 32), retired AS (INSERT INTO game_world_raid_retirements SELECT firing_id,$2,'world_event_stale' FROM stale ON CONFLICT DO NOTHING RETURNING firing_id) UPDATE game_world_raid_firings f SET state=3 FROM retired WHERE f.firing_id=retired.firing_id AND f.state=1")
        .bind(world.as_bytes().as_slice()).bind(now).execute(&mut **tx).await?;
    Ok(())
}

/// A reset or retired admission must not strand an armed object forever when
/// no Channel was able to report its physical placement.
pub(crate) async fn reconcile_hatches(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    state: &mut EventState,
    now: i64,
) -> Result<(), DurabilityError> {
    let now = now.max(state.observed_ms);
    let mut changed = false;
    for (&member, &firing) in &state.lifecycle.last_firing {
        let object = state
            .lifecycle
            .objects
            .get(&member)
            .ok_or(DurabilityError::InvalidStoredState)?;
        if state.lifecycle.firing_versions.get(&member) != Some(&object.revision()) {
            continue;
        }
        let facts: Option<(i16,String,bool)> = sqlx::query_as("SELECT state,reset_epoch::text,EXISTS(SELECT 1 FROM game_world_raid_retirements r WHERE r.firing_id=f.firing_id) FROM game_world_raid_firings f WHERE firing_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
            .bind(firing.as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let Some((status, prior_epoch, retired)) = facts else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if status == 3 || retired || prior_epoch != epoch.to_string() {
            let object = state
                .lifecycle
                .objects
                .get_mut(&member)
                .ok_or(DurabilityError::InvalidStoredState)?;
            object
                .reset()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            object
                .begin(now, oteryn_gameplay_world_change::timed::HOUR)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            changed = true;
        }
    }
    if changed {
        save(
            tx,
            world,
            EventKind::SpringIntoLife,
            state,
            cycle(EventKind::SpringIntoLife, epoch, state.start_ms)?,
        )
        .await?;
    }
    Ok(())
}
