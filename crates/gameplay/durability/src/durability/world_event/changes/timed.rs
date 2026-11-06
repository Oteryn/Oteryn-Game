//! Timed state joins the reset owner's current-epoch fence and the caller transaction.
use super::*;
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use oteryn_gameplay_world_change::timed::{EventKind, EventState, Input, MINUTE};

/// Internal semantic input. Transport payloads must never supply clock/draw or credited kills.
#[derive(Debug, Clone, Copy)]
pub struct TimedRequest {
    pub world: WorldId,
    pub epoch: u64,
    pub kind: EventKind,
    pub now_ms: i64,
    pub operation: [u8; 16],
    pub input: Input,
}

/// The same live session/node/generation proof as an ordinary quest mutation.
pub struct TimedActor<'a> {
    pub recovery: &'a CharacterRecoveryFenceV1,
    pub node: &'a NodeIncarnationProof,
    pub fence: CurrentCharacterGameplayFence,
}

async fn window(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    kind: EventKind,
    now: i64,
) -> Result<Option<(i64, i64)>, DurabilityError> {
    if now < 0 || super::load(tx, world, epoch).await?.is_none() {
        return Ok(None);
    }
    let bounds: (i64, i64) = if let Some((start, end)) = kind.window() {
        sqlx::query_as("SELECT (extract(epoch FROM make_date(extract(year FROM to_timestamp($1::double precision/1000) AT TIME ZONE 'UTC')::int,$2,$3)::timestamp AT TIME ZONE 'UTC')*1000)::bigint,(extract(epoch FROM make_date(extract(year FROM to_timestamp($1::double precision/1000) AT TIME ZONE 'UTC')::int,$4,$5)::timestamp AT TIME ZONE 'UTC')*1000)::bigint")
            .bind(now).bind(i32::from(start.0)).bind(i32::from(start.1)).bind(i32::from(end.0)).bind(i32::from(end.1)).fetch_one(&mut **tx).await?
    } else {
        let start: i64 = sqlx::query_scalar("SELECT (extract(epoch FROM activated_at)*1000)::bigint FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
        (
            start,
            start
                .checked_add(86_400_000)
                .ok_or(DurabilityError::InvalidStoredState)?,
        )
    };
    Ok((bounds.0 <= now).then_some(bounds))
}

fn cycle(kind: EventKind, epoch: u64, start: i64) -> Result<i64, DurabilityError> {
    if kind.window().is_some() {
        Ok(start)
    } else {
        i64::try_from(epoch).map_err(|_| DurabilityError::InvalidStoredState)
    }
}

async fn state(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    kind: EventKind,
    bounds: (i64, i64),
    cycle: i64,
) -> Result<EventState, DurabilityError> {
    let initial = serde_json::to_value(EventState::new(bounds.0, bounds.1))
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_world_timed_events VALUES(encode($1,'hex')::uuid,$2,$3,$4) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(cycle).bind(initial).execute(&mut **tx).await?;
    let value = sqlx::query_scalar("SELECT state FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(cycle).fetch_one(&mut **tx).await?;
    let state: EventState =
        serde_json::from_value(value).map_err(|_| DurabilityError::InvalidStoredState)?;
    if state.start_ms != bounds.0 || state.end_ms != bounds.1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(state)
}

async fn save(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    kind: EventKind,
    state: &EventState,
    cycle: i64,
) -> Result<(), DurabilityError> {
    let value = serde_json::to_value(state).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("UPDATE game_world_timed_events SET state=$4 WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3")
        .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(cycle).bind(value).execute(&mut **tx).await?;
    Ok(())
}

/// Scheduler/boot projection. One World row, independent of Channel and restart.
/// No missed-wave backlog is spawned; the current synchronized slot is projected.
pub async fn poll(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    kind: EventKind,
    now: i64,
) -> Result<Option<EventState>, DurabilityError> {
    let Some(bounds) = window(tx, world, epoch, kind, now).await? else {
        return Ok(None);
    };
    let cycle = cycle(kind, epoch, bounds.0)?;
    let mut state = state(tx, world, kind, bounds, cycle).await?;
    // Channel clocks can arrive out of order; never rewind a committed projection.
    let now = now.max(state.observed_ms);
    state
        .advance(kind, now, Input::Tick, 0)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    save(tx, world, kind, &state, cycle).await?;
    Ok(Some(state))
}

/// A pending result commits with the quest's custody debit and physical-effect receipt.
/// Failed jar attempts record their cooldown; exact retries retain their first outcome.
pub async fn interact(
    tx: &mut Transaction<'_, Postgres>,
    actor: TimedActor<'_>,
    request: TimedRequest,
) -> Result<Option<bool>, DurabilityError> {
    assert_recovery_fence(tx, actor.recovery).await?;
    lock_admission_relations(tx).await?;
    if assert_gameplay_fence(tx, &actor.fence, actor.node)
        .await?
        .is_err()
        || actor.fence.runtime_scope.world_id() != request.world
        || request.operation == [0; 16]
        || request.input == Input::Tick
    {
        return Ok(None);
    }
    let Some(bounds) = window(
        tx,
        request.world,
        request.epoch,
        request.kind,
        request.now_ms,
    )
    .await?
    else {
        return Ok(None);
    };
    if request.now_ms >= bounds.1 {
        return Ok(None);
    }
    let cycle = cycle(request.kind, request.epoch, bounds.0)?;
    let mut state = state(tx, request.world, request.kind, bounds, cycle).await?;
    let character = actor.fence.character_id;
    let input =
        serde_json::to_value(request.input).map_err(|_| DurabilityError::InvalidStoredState)?;
    let receipt: Option<(serde_json::Value, bool)> = sqlx::query_as("SELECT input,applied FROM game_world_timed_receipts WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 AND character_id=encode($4,'hex')::uuid AND operation_id=$5")
        .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).bind(request.operation.as_slice()).fetch_optional(&mut **tx).await?;
    if let Some((previous, applied)) = receipt {
        return Ok((previous == input).then_some(applied));
    }
    sqlx::query("INSERT INTO game_world_timed_participants(world_id,event_key,cycle_id,character_id) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid) ON CONFLICT DO NOTHING")
        .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).execute(&mut **tx).await?;
    let ready: i64 = sqlx::query_scalar("SELECT ready_ms FROM game_world_timed_participants WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 AND character_id=encode($4,'hex')::uuid FOR UPDATE")
        .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if request.now_ms < ready || request.now_ms < state.observed_ms {
        return Ok(None);
    }
    let mut hash = Sha256::new();
    hash.update(b"oteryn.world_event.timed.v1\0");
    hash.update(request.world.as_bytes());
    hash.update(request.kind.key());
    hash.update(cycle.to_be_bytes());
    let root = GameplayDecisionRoot::from_bytes(hash.finalize().into());
    let draw = deterministic_decision_u64(
        &root,
        DecisionOccurrenceId::from_bytes(request.operation),
        "interaction",
        0,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let before = state.objects;
    let applied = state
        .advance(request.kind, request.now_ms, request.input, draw)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let cooldown = if matches!(request.input, Input::Jar(aid) if aid == 50006 + u16::from(before))
        && !applied
        && before < 4
    {
        request
            .now_ms
            .checked_add(30 * MINUTE)
            .ok_or(DurabilityError::InvalidStoredState)?
    } else {
        0
    };
    save(tx, request.world, request.kind, &state, cycle).await?;
    sqlx::query("UPDATE game_world_timed_participants SET credited=credited+$5,ready_ms=$6 WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 AND character_id=encode($4,'hex')::uuid")
        .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).bind(i64::from(applied)).bind(cooldown).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_world_timed_receipts VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5,$6,$7)")
        .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).bind(request.operation.as_slice()).bind(input).bind(applied).execute(&mut **tx).await?;
    Ok(Some(applied))
}
