//! Timed state joins the reset owner's current-epoch fence and the caller transaction.
use super::*;
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use oteryn_gameplay_world_change::timed::MINUTE;
pub use oteryn_gameplay_world_change::timed::{EventKind, EventState, Input};

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

fn qualified(kind: EventKind) -> bool {
    static KEYS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| {
        let Ok(catalogue) = serde_json::from_str::<serde_json::Value>(CATALOGUE_JSON) else {
            return Vec::new();
        };
        let Some(entries) = catalogue["entries"]
            .as_array()
            .filter(|entries| entries.len() <= 256)
        else {
            return Vec::new();
        };
        EventKind::ALL
            .into_iter()
            .filter_map(|kind| {
                let key = if kind == EventKind::MummysCurse {
                    "the_mummy_s_curse"
                } else {
                    kind.key()
                };
                entries
                    .iter()
                    .any(|entry| {
                        entry["key"] == key
                            && entry["status"] == "QUALIFIED"
                            && entry["trigger"]["kind"] == "timed"
                            && entry["trigger"]["event"] == kind.key()
                            && entry["trigger"]["owner"] == "world_event::changes::timed"
                    })
                    .then_some(kind.key())
            })
            .collect()
    })
    .contains(&kind.key())
}

async fn window(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    kind: EventKind,
    now: i64,
) -> Result<Option<(i64, i64)>, DurabilityError> {
    if now < 0 || !qualified(kind) {
        return Ok(None);
    }
    let Some(day) = super::load(tx, world, epoch).await? else {
        return Ok(None);
    };
    let selected = match kind {
        EventKind::SpringIntoLife => day.timed.spring,
        EventKind::RiseOfDevovorga => day.timed.rise,
        EventKind::DevovorgasEssence => day.timed.essence,
        EventKind::Kingsday => day.timed.kingsday,
        EventKind::InsectoidInvasion => day.timed.insectoid,
        _ => true,
    };
    if !selected {
        return Ok(None);
    }
    // The donor starts Flower Month on a June startup/server save and then
    // cycles until reset, including the final June day's overnight tail.
    if kind == EventKind::FlowerMonth && !day.flower_month {
        return Ok(None);
    }
    let bounds: (i64, i64) = if let Some((start, end)) =
        kind.window().filter(|_| kind != EventKind::FlowerMonth)
    {
        sqlx::query_as("SELECT (extract(epoch FROM ((make_date(extract(year FROM to_timestamp($1::double precision/1000) AT TIME ZONE 'UTC')::int,$2,$3)::timestamp + CASE WHEN $6 THEN interval '6 hours' ELSE interval '0 hours' END) AT TIME ZONE CASE WHEN $6 THEN 'Europe/Berlin' ELSE 'UTC' END))*1000)::bigint,(extract(epoch FROM ((make_date(extract(year FROM to_timestamp($1::double precision/1000) AT TIME ZONE 'UTC')::int,$4,$5)::timestamp + CASE WHEN $6 THEN interval '6 hours' ELSE interval '0 hours' END) AT TIME ZONE CASE WHEN $6 THEN 'Europe/Berlin' ELSE 'UTC' END))*1000)::bigint")
            .bind(now).bind(i32::from(start.0)).bind(i32::from(start.1)).bind(i32::from(end.0)).bind(i32::from(end.1)).bind(kind.server_save_window()).fetch_one(&mut **tx).await?
    } else {
        sqlx::query_as("SELECT (extract(epoch FROM activated_at)*1000)::bigint,(extract(epoch FROM ((date_trunc('day',activated_at AT TIME ZONE 'Europe/Berlin'-interval '6 hours')+interval '1 day 6 hours') AT TIME ZONE 'Europe/Berlin'))*1000)::bigint FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?
    };
    Ok((bounds.0 <= now).then_some(bounds))
}

fn cycle(kind: EventKind, epoch: u64, start: i64) -> Result<i64, DurabilityError> {
    if kind.window().is_some() && kind != EventKind::FlowerMonth {
        Ok(start)
    } else {
        i64::try_from(epoch).map_err(|_| DurabilityError::InvalidStoredState)
    }
}

/// Also protects reconciliation when the first state row is still uncommitted.
async fn lock_cycle(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    kind: EventKind,
    cycle: i64,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:world-timed:'||encode($1,'hex')||':'||$2||':'||$3::bigint::text,0))")
        .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(cycle)
        .execute(&mut **tx).await?;
    Ok(())
}

async fn state(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    kind: EventKind,
    bounds: (i64, i64),
    cycle: i64,
    now: i64,
) -> Result<EventState, DurabilityError> {
    lock_cycle(tx, world, kind, cycle).await?;
    let activation = if kind == EventKind::FlowerMonth && now < bounds.1 {
        now
    } else {
        bounds.0
    };
    let initial = serde_json::to_value(
        EventState::for_kind(kind, activation, bounds.1)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_world_timed_events VALUES(encode($1,'hex')::uuid,$2,$3,$4) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(cycle).bind(initial).execute(&mut **tx).await?;
    let value = sqlx::query_scalar("SELECT state FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(cycle).fetch_one(&mut **tx).await?;
    let state: EventState =
        serde_json::from_value(value).map_err(|_| DurabilityError::InvalidStoredState)?;
    let start_matches = if kind == EventKind::FlowerMonth {
        bounds.0 <= state.start_ms && state.start_ms < bounds.1
    } else {
        state.start_ms == bounds.0
    };
    state
        .lifecycle
        .validate()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if !start_matches || state.end_ms != bounds.1 {
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

/// The normal source is PostgreSQL wall time. Only isolated test roots can
/// supply an override; no caller or transport field carries a date policy.
pub(crate) async fn event_now_ms(
    tx: &mut Transaction<'_, Postgres>,
    clock: Option<i64>,
) -> Result<i64, DurabilityError> {
    if let Some(now) = clock {
        return Ok(now);
    }
    Ok(
        sqlx::query_scalar("SELECT (extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?,
    )
}

async fn expire_windows(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    kind: EventKind,
    now: i64,
) -> Result<(), DurabilityError> {
    let cycles:Vec<i64>=sqlx::query_scalar("SELECT cycle_id FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND state->>'phase' IN('Active','Dormant') AND (state->>'end_ms')::bigint<=$3 ORDER BY (state->>'end_ms')::bigint,cycle_id LIMIT 8")
        .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(now).fetch_all(&mut **tx).await?;
    for cycle in cycles {
        lock_cycle(tx, world, kind, cycle).await?;
        let raw=sqlx::query_scalar("SELECT state FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 FOR UPDATE")
            .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(cycle).fetch_one(&mut **tx).await?;
        let mut state: EventState =
            serde_json::from_value(raw).map_err(|_| DurabilityError::InvalidStoredState)?;
        state
            .lifecycle
            .validate()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if state.end_ms <= now {
            state
                .advance(kind, now.max(state.observed_ms), Input::Tick, 0)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            save(tx, world, kind, &state, cycle).await?;
        }
    }
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
    if now < 0 || !qualified(kind) || super::load(tx, world, epoch).await?.is_none() {
        return Ok(None);
    }
    expire_windows(tx, world, kind, now).await?;
    poll_current(tx, world, epoch, kind, now).await
}

// A Character interaction already owns its current cycle. It must not take
// older-cycle locks after that; only the scheduler performs expiry paging.
async fn poll_current(
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
    let mut state = state(tx, world, kind, bounds, cycle, now).await?;
    // Channel clocks can arrive out of order; never rewind a committed projection.
    let now = now.max(state.observed_ms);
    let mut hash = Sha256::new();
    hash.update(b"oteryn.world_event.timed.v1\0");
    hash.update(world.as_bytes());
    hash.update(kind.key());
    hash.update(cycle.to_be_bytes());
    let root = GameplayDecisionRoot::from_bytes(hash.finalize().into());
    let mut occurrence = [0; 16];
    occurrence[8..].copy_from_slice(
        &((now - state.start_ms).div_euclid(oteryn_gameplay_world_change::timed::HOUR))
            .to_be_bytes(),
    );
    let draw = deterministic_decision_u64(
        &root,
        DecisionOccurrenceId::from_bytes(occurrence),
        "hourly_trigger",
        0,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    state
        .advance(kind, now, Input::Tick, draw)
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
        || matches!(request.input, Input::Tick | Input::Defeat(_))
    {
        return Ok(None);
    }
    // Ordinary progression also permits reconnectable sessions. A jar USE needs
    // a live controller, rechecked under the session row lock before receipt replay.
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND session_state=2 AND current_transport_ref IS NOT NULL)")
        .bind(actor.fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if !active {
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
    let mut state = state(
        tx,
        request.world,
        request.kind,
        bounds,
        cycle,
        request.now_ms,
    )
    .await?;
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
    let (ready, ready_object): (i64, i32) = sqlx::query_as("SELECT ready_ms,ready_object FROM game_world_timed_participants WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 AND character_id=encode($4,'hex')::uuid FOR UPDATE")
        .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let object = match request.input {
        Input::Jar(aid) | Input::BeginObject(aid) | Input::Extinguish(aid) | Input::Fill(aid) => {
            i32::from(aid)
        }
        Input::Enter(member) => i32::from(member),
        _ => 0,
    };
    if (request.kind == EventKind::MummysCurse && request.now_ms < ready && ready_object == object)
        || request.now_ms < state.observed_ms
    {
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
    if matches!(request.input, Input::Reward | Input::Attend) {
        let (mut credited,claimed,reward_ready):(i64,bool,bool)=sqlx::query_as("SELECT credited,claimed,reward_ready FROM game_world_timed_participants WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 AND character_id=encode($4,'hex')::uuid")
            .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if request.input == Input::Attend {
            if request.kind != EventKind::Kingsday || claimed {
                return Ok(None);
            }
            if !state
                .advance(request.kind, request.now_ms, Input::Attend, draw)
                .map_err(|_| DurabilityError::InvalidStoredState)?
            {
                return Ok(None);
            }
            credited = credited
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            sqlx::query("UPDATE game_world_timed_participants SET credited=$5 WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 AND character_id=encode($4,'hex')::uuid")
                .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).bind(credited).execute(&mut **tx).await?;
            save(tx, request.world, request.kind, &state, cycle).await?;
        }
        if !state.reward_admitted(
            u64::try_from(credited).map_err(|_| DurabilityError::InvalidStoredState)?,
            claimed,
        ) || (request.kind == EventKind::SpringIntoLife
            && !reward_ready
            && !u16::try_from(ready_object)
                .ok()
                .and_then(|member| state.lifecycle.last_success_ms.get(&member))
                .is_some_and(|at| ready > 0 && *at >= ready))
        {
            return Ok(None);
        }
        sqlx::query("UPDATE game_world_timed_participants SET claimed=true WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 AND character_id=encode($4,'hex')::uuid")
            .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_world_timed_receipts(world_id,event_key,cycle_id,character_id,operation_id,input,applied) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5,$6,true)")
            .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).bind(request.operation.as_slice()).bind(input).execute(&mut **tx).await?;
        return Ok(Some(true));
    }
    let earned_hatch = request.kind == EventKind::SpringIntoLife
        && u16::try_from(ready_object)
            .ok()
            .and_then(|member| state.lifecycle.last_success_ms.get(&member))
            .is_some_and(|at| ready > 0 && *at >= ready);
    let before = state.objects;
    let mut applied = state
        .advance(request.kind, request.now_ms, request.input, draw)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    // Joining an already armed public object does not restart its shared clock.
    if request.kind == EventKind::SpringIntoLife
        && let Input::BeginObject(member) = request.input
        && let Some(at) = state
            .lifecycle
            .objects
            .get(&member)
            .and_then(|o| o.ready_at())
        && at > request.now_ms
        && (ready != at || ready_object != object)
    {
        applied = true;
    }
    let expected_jar = matches!(request.input, Input::Jar(aid) if aid == 50006 + u16::from(before));
    let (cooldown, cooldown_object) = if request.kind == EventKind::SpringIntoLife
        && applied
        && matches!(request.input, Input::BeginObject(_))
    {
        (
            state
                .lifecycle
                .objects
                .get(&u16::try_from(object).map_err(|_| DurabilityError::InvalidStoredState)?)
                .and_then(|o| o.ready_at())
                .ok_or(DurabilityError::InvalidStoredState)?,
            object,
        )
    } else if !expected_jar {
        // An out-of-order jar must not clear another jar's failed-attempt cooldown.
        (ready, ready_object)
    } else if !applied && before < 4 {
        (
            request
                .now_ms
                .checked_add(30 * MINUTE)
                .ok_or(DurabilityError::InvalidStoredState)?,
            object,
        )
    } else {
        (0, object)
    };
    save(tx, request.world, request.kind, &state, cycle).await?;
    sqlx::query("UPDATE game_world_timed_participants SET credited=credited+$5,ready_ms=$6,ready_object=$7,reward_ready=reward_ready OR $8 WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 AND character_id=encode($4,'hex')::uuid")
        .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).bind(i64::from(applied && matches!(request.input,Input::Jar(_)|Input::BeginObject(_)|Input::Extinguish(_)|Input::Fill(_)))).bind(cooldown).bind(cooldown_object).bind(earned_hatch).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_world_timed_receipts(world_id,event_key,cycle_id,character_id,operation_id,input,applied) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5,$6,$7)")
        .bind(request.world.as_bytes().as_slice()).bind(request.kind.key()).bind(cycle).bind(character.as_bytes().as_slice()).bind(request.operation.as_slice()).bind(input).bind(applied).execute(&mut **tx).await?;
    Ok(Some(applied))
}

/// Calendar-only projection, sampled in the movement owner turn.
pub fn seasonal_open(kind: EventKind, unix_ms: i64) -> bool {
    if !qualified(kind) {
        return false;
    }
    let Some((start, end)) = kind.window() else {
        return false;
    };
    let (month, day) = super::utc_month_day(unix_ms);
    oteryn_gameplay_world_change::owner::SeasonalWindow { start, end }.contains(month, day)
}
// Native USE and the existing Channel effect loop call these wrappers.
impl DurabilityRoot {
    pub async fn use_mummy_jar(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        epoch: u64,
        command: crate::foundation::CommandRef,
        action: u16,
    ) -> Result<Option<(bool, EventState)>, DurabilityError> {
        if !(50006..=50010).contains(&action) || command.game_session_id() != fence.game_session_id
        {
            return Ok(None);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let clock = self.world_event_clock_override_ms();
        let mut hash = Sha256::new();
        hash.update(b"oteryn:wevents:use:v1\0");
        hash.update(command.game_session_id().as_bytes());
        hash.update(command.command_id().get().to_be_bytes());
        let mut operation = [0; 16];
        operation.copy_from_slice(&hash.finalize()[..16]);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let now_ms = event_now_ms(&mut tx, clock).await?;
                    let request = TimedRequest {
                        world: fence.runtime_scope.world_id(),
                        epoch,
                        kind: EventKind::MummysCurse,
                        now_ms,
                        operation,
                        input: Input::Jar(action),
                    };
                    let Some(applied) = interact(
                        &mut tx,
                        TimedActor {
                            recovery: &recovery,
                            node: &node,
                            fence,
                        },
                        request,
                    )
                    .await?
                    else {
                        return Ok(None);
                    };
                    let Some(state) =
                        poll_current(&mut tx, request.world, epoch, request.kind, now_ms).await?
                    else {
                        return Ok(None);
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some((applied, state)))
                })
            })
            .await
    }

    /// This is observation, not a grant to an actor or a Channel generation.
    pub async fn timed_world_event(
        &self,
        world: WorldId,
        epoch: u64,
        kind: EventKind,
    ) -> Result<Option<EventState>, DurabilityError> {
        let clock = self.world_event_clock_override_ms();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let now_ms=event_now_ms(&mut tx,clock).await?;
            let state=if let Some(bounds)=window(&mut tx,world,epoch,kind,now_ms).await? {
                let cycle=cycle(kind,epoch,bounds.0)?;
                let stored:Option<serde_json::Value>=sqlx::query_scalar("SELECT state FROM game_world_timed_events WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3")
                    .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(cycle).fetch_optional(&mut *tx).await?;
                stored.map(serde_json::from_value).transpose().map_err(|_|DurabilityError::InvalidStoredState)?
            } else {None};
            commit_semantic_transaction(tx,deadline).await?;
            Ok(state)
        })).await
    }
}

impl DurabilityRoot {
    /// Unknown-commit reconciliation reads history; it never opens an encounter or grants authority.
    pub async fn reconcile_mummy_use(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: crate::domain::CharacterId,
        world: WorldId,
        epoch: u64,
        command: crate::foundation::CommandRef,
        action: u16,
    ) -> Result<Option<bool>, DurabilityError> {
        if !(50006..=50010).contains(&action) {
            return Ok(None);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let mut hash = Sha256::new();
        hash.update(b"oteryn:wevents:use:v1\0");
        hash.update(command.game_session_id().as_bytes());
        hash.update(command.command_id().get().to_be_bytes());
        let mut operation = [0; 16];
        operation.copy_from_slice(&hash.finalize()[..16]);
        let cycle = i64::try_from(epoch).map_err(|_| DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_cycle(&mut tx, world, EventKind::MummysCurse, cycle).await?;
            let receipt:Option<(serde_json::Value,bool)>=sqlx::query_as("SELECT input,applied FROM game_world_timed_receipts WHERE world_id=encode($1,'hex')::uuid AND event_key='mummys_curse' AND cycle_id=$2 AND character_id=encode($3,'hex')::uuid AND operation_id=$4")
                .bind(world.as_bytes().as_slice()).bind(cycle).bind(character.as_bytes().as_slice()).bind(operation.as_slice()).fetch_optional(&mut *tx).await?;
            let expected=serde_json::to_value(Input::Jar(action)).map_err(|_|DurabilityError::InvalidStoredState)?;
            let result=receipt.and_then(|(input,applied)|(input==expected).then_some(applied));
            commit_semantic_transaction(tx,deadline).await?;
            Ok(result)
        })).await
    }
}

/// Called only by the fenced Quest writer, in the same transaction as custody
/// debits, storage changes and MINT. Transport supplies none of this policy.
pub(crate) async fn apply_binding(
    tx: &mut Transaction<'_, Postgres>,
    actor: TimedActor<'_>,
    binding: &oteryn_gameplay_world_change::binding::Binding,
    epoch: Option<u64>,
    level: i64,
    request: &crate::durability::quest_state::QuestTransitionRequest,
    clock: Option<i64>,
) -> Result<bool, DurabilityError> {
    use crate::durability::quest_state::QuestCause;
    if !binding.valid() || !u32::try_from(level).is_ok_and(|level| binding.level_admitted(level)) {
        return Ok(false);
    }
    let (command, namespace) = match request.cause {
        QuestCause::Command(c) => (c, "command"),
        QuestCause::Use(c) => (c, "use"),
        QuestCause::Step(c) => (c, "step"),
        _ => return Ok(false),
    };
    let world = actor.fence.runtime_scope.world_id();
    let epoch = if let Some(epoch) = epoch {
        epoch
    } else {
        let Some(value): Option<String> = sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid")
            .bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await? else { return Ok(false); };
        value
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?
    };
    let now_ms = event_now_ms(tx, clock).await?;
    let mut hash = Sha256::new();
    hash.update(b"oteryn:wevents:quest:v1\0");
    hash.update(namespace.as_bytes());
    hash.update(command.game_session_id().as_bytes());
    hash.update(command.command_id().get().to_be_bytes());
    hash.update(request.transition_key.as_bytes());
    let mut operation = [0; 16];
    operation.copy_from_slice(&hash.finalize()[..16]);
    Ok(interact(
        tx,
        actor,
        TimedRequest {
            world,
            epoch,
            kind: binding.event,
            now_ms,
            operation,
            input: binding.action,
        },
    )
    .await?
        == Some(true))
}

#[path = "timed/raids.rs"]
mod raids;
pub use raids::{NativeKillOutcome, NativeKillRequest, RaidOrigin};
pub(crate) use raids::{raid_origin, seal_raid_origin};

pub(crate) mod schedules;
