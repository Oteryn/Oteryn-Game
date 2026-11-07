//! World-owned credits join one existing semantic transaction. A receipt and all
//! affected counters commit together; callbacks never commit the caller's transaction.
pub mod benevola;
mod fire_earth;
pub(crate) mod fire_feathered;
pub(super) mod shared_objects;
pub(crate) mod shared_progress;

use super::*;
use crate::durability::character_progression::assert_runtime_scope_fence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{CharacterId, RuntimeScopeRefV1, ScopeOwnershipGeneration};
pub use oteryn_gameplay_world_change::counter::{Bindings, CounterKind, CounterState, Credit};
use sqlx::Row;
use std::sync::OnceLock;

pub fn bindings() -> Result<&'static Bindings, DurabilityError> {
    static DATA: OnceLock<Result<Bindings, &'static str>> = OnceLock::new();
    DATA.get_or_init(|| {
        let data: Bindings =
            serde_json::from_str(oteryn_gameplay_world_change::counter::BINDINGS_JSON)
                .map_err(|_| "counter bindings")?;
        data.validate().map_err(|_| "counter bounds")?;
        Ok(data)
    })
    .as_ref()
    .map_err(|_| DurabilityError::InvalidStoredState)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreditOutcome {
    Applied { phase_changed: bool },
    AlreadyApplied,
    Refused,
}
/// Source events are emitted by native death, quest-receipt or item-burn adapters.
/// A client cannot select a counter, quantity or occurrence through this API.
#[derive(Debug, Clone)]
pub struct CreditEvent {
    pub source_kind: u8,
    pub occurrence: Vec<u8>,
    pub credits: Vec<(CounterKind, Credit)>,
    pub counter_generations: [u64; 8],
    pub occurred_at_ms: u64,
    pub character: Option<CharacterId>,
}
pub struct CreditAuthority {
    transaction: String,
    recovery: CharacterRecoveryFenceV1,
    node: NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    epoch: u64,
}
/// Resolve current scope/node and epoch independently of the retained source event.
#[allow(
    clippy::too_many_arguments,
    reason = "every independently resolved authority fact is explicit"
)]
pub async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    epoch: u64,
) -> Result<Option<CreditAuthority>, DurabilityError> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    if !assert_runtime_scope_fence(tx, scope, generation, node).await?
        || !super::load(tx, scope.world_id(), epoch)
            .await?
            .is_some_and(|day| day.world_counters)
    {
        return Ok(None);
    }
    let transaction = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    Ok(Some(CreditAuthority {
        transaction,
        recovery: recovery.clone(),
        node: node.clone(),
        scope,
        generation,
        epoch,
    }))
}
impl CreditAuthority {
    async fn current(&self, tx: &mut Transaction<'_, Postgres>) -> Result<bool, DurabilityError> {
        let transaction: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut **tx)
            .await?;
        if transaction != self.transaction {
            return Ok(false);
        }
        assert_recovery_fence(tx, &self.recovery).await?;
        lock_admission_relations(tx).await?;
        Ok(
            assert_runtime_scope_fence(tx, self.scope, self.generation, &self.node).await?
                && super::load(tx, self.scope.world_id(), self.epoch)
                    .await?
                    .is_some_and(|day| day.world_counters),
        )
    }
}
async fn states(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    lock: bool,
) -> Result<[CounterState; 8], DurabilityError> {
    let statement = if lock {
        "SELECT change_key,reset_epoch::text,state FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid ORDER BY change_key LIMIT 9 FOR UPDATE"
    } else {
        "SELECT change_key,reset_epoch::text,state FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid ORDER BY change_key LIMIT 9"
    };
    let rows = sqlx::query(statement)
        .bind(world.as_bytes().as_slice())
        .fetch_all(&mut **tx)
        .await?;
    if rows.len() != 8 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut values = [CounterState::new(CounterKind::Deeplings, 0, false); 8];
    let mut seen = [false; 8];
    for row in rows {
        let key: String = row.try_get("change_key")?;
        let kind = CounterKind::from_key(&key).ok_or(DurabilityError::InvalidStoredState)?;
        let value: CounterState = serde_json::from_value(row.try_get("state")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let epoch: String = row.try_get("reset_epoch")?;
        value
            .validate(kind)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if seen[kind.index()] || epoch != value.epoch.to_string() {
            return Err(DurabilityError::InvalidStoredState);
        }
        seen[kind.index()] = true;
        values[kind.index()] = value;
    }
    Ok(values)
}
async fn store(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    kind: CounterKind,
    state: CounterState,
) -> Result<(), DurabilityError> {
    state
        .validate(kind)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let json = serde_json::to_value(state).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("UPDATE game_world_change_counters SET reset_epoch=$3::text::numeric,state=$4 WHERE world_id=encode($1,'hex')::uuid AND change_key=$2")
        .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(state.epoch.to_string()).bind(json).execute(&mut **tx).await?;
    Ok(())
}
/// Called only by the existing reset/day owner under its admission relation lock.
pub(super) async fn at_reset(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    earth_active: bool,
    now_ms: u64,
) -> Result<(), DurabilityError> {
    let parameters = &bindings()?.parameters;
    for kind in CounterKind::ALL {
        let initial = CounterState::new(kind, epoch, earth_active);
        sqlx::query("INSERT INTO game_world_change_counters(world_id,change_key,reset_epoch,state) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4) ON CONFLICT DO NOTHING")
            .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(epoch.to_string()).bind(serde_json::to_value(initial).map_err(|_|DurabilityError::InvalidStoredState)?)
            .execute(&mut **tx).await?;
    }
    let before = states(tx, world, true).await?;
    for kind in CounterKind::ALL {
        let old = before[kind.index()];
        let next = old
            .at_save(kind, epoch, earth_active, parameters, now_ms)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if next != old {
            store(tx, world, kind, next).await?;
        }
    }
    Ok(())
}
pub(super) async fn project(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    day: &mut DailyWorldChanges,
) -> Result<(), DurabilityError> {
    if !day.world_counters {
        return Ok(());
    }
    let now_ms: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let now_ms = u64::try_from(now_ms).map_err(|_| DurabilityError::InvalidStoredState)?;
    day.shared_used_sources_mask =
        shared_objects::used_sources_mask(tx, world, day.reset_epoch).await?;
    let values = states(tx, world, false).await?;
    for kind in CounterKind::ALL {
        let value = values[kind.index()];
        if value.epoch != day.reset_epoch {
            return Err(DurabilityError::InvalidStoredState);
        }
        let value = value
            .tick(kind, now_ms)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        day.counter_stages[kind.index()] = value.stage;
        day.counter_progress[kind.index()] = value.counts;
        day.counter_generations[kind.index()] = value.credit_generation;
    }
    Ok(())
}
/// Joins the producer's transaction. Validate everything before inserting a receipt;
/// a refused event leaves both receipts and counters unchanged, even if caller commits.
pub async fn credit(
    tx: &mut Transaction<'_, Postgres>,
    authority: &CreditAuthority,
    event: &CreditEvent,
) -> Result<CreditOutcome, DurabilityError> {
    if !(1..=5).contains(&event.source_kind)
        || !(16..=128).contains(&event.occurrence.len())
        || event.credits.is_empty()
        || event.credits.len() > 8
        || !authority.current(tx).await?
    {
        return Ok(CreditOutcome::Refused);
    }
    let source_matches = event.credits.iter().all(|(_, credit)| {
        matches!(
            (event.source_kind, credit),
            (
                1,
                Credit::Kill(_) | Credit::VoiceServant { .. } | Credit::VoiceMage
            ) | (
                2,
                Credit::Task(_)
                    | Credit::HeartDestroyed
                    | Credit::SoulNetDestroyed
                    | Credit::CoralPathCompleted
                    | Credit::StairPathCompleted
            ) | (3, Credit::TrappedWolves(_))
                | (4, Credit::MissionComplete)
        )
    });
    if !source_matches || event.character.is_none() {
        return Ok(CreditOutcome::Refused);
    }
    if event.source_kind == 1 {
        let mut prefix = Vec::new();
        match authority.scope {
            RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } => {
                prefix.extend_from_slice(world_id.as_bytes());
                prefix.extend_from_slice(channel_id.as_bytes());
            }
            RuntimeScopeRefV1::Instance {
                world_id,
                instance_id,
            } => {
                prefix.push(2);
                prefix.extend_from_slice(world_id.as_bytes());
                prefix.extend_from_slice(&instance_id);
            }
        }
        prefix.extend_from_slice(&authority.generation.get().to_be_bytes());
        if !event.occurrence.starts_with(&prefix) || event.occurrence.len() != prefix.len() + 12 {
            return Ok(CreditOutcome::Refused);
        }
    }
    let world = authority.scope.world_id();
    // Current Character roots precede World counter rows everywhere. This also keeps
    // the personal observer's FK/write lock order compatible with quest callbacks.
    let character = if let Some(character) = event.character {
        let exists:Option<i32>=sqlx::query_scalar("SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(character.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        exists.map(|_| character)
    } else {
        None
    };
    if character.is_none() {
        return Ok(CreditOutcome::Refused);
    }
    let mut values = states(tx, world, true).await?;
    let serialized = serde_json::to_vec(&(
        authority.epoch,
        event.source_kind,
        &event.occurrence,
        &event.credits,
        event.counter_generations,
        event.occurred_at_ms,
        // The receipt key is the full World death. Its payload also freezes the
        // credited principal: substitution conflicts instead of rebinding progress.
        event.character.map(|character| *character.as_bytes()),
    ))
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let binding: Vec<u8> = Sha256::digest(serialized).to_vec();
    let previous:Option<Vec<u8>>=sqlx::query_scalar("SELECT binding_digest FROM game_world_change_credit_receipts WHERE world_id=encode($1,'hex')::uuid AND source_kind=$2 AND occurrence=$3")
        .bind(world.as_bytes().as_slice()).bind(i16::from(event.source_kind)).bind(&event.occurrence).fetch_optional(&mut **tx).await?;
    if let Some(previous) = previous {
        return Ok(if previous == binding {
            CreditOutcome::AlreadyApplied
        } else {
            CreditOutcome::Refused
        });
    }
    // Recheck live authority after any wait for Character/counter row locks.
    if !authority.current(tx).await? {
        return Ok(CreditOutcome::Refused);
    }
    let current_ms: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let current_ms = u64::try_from(current_ms).map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut seen = [false; 8];
    let mut changed = false;
    for &(kind, credit) in &event.credits {
        let index = kind.index();
        let before = values[index]
            .tick(kind, current_ms)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if seen[index]
            || before.epoch != authority.epoch
            || before.credit_generation != event.counter_generations[index]
        {
            return Ok(CreditOutcome::Refused);
        }
        seen[index] = true;
        let Ok(after) = before.credit(kind, credit, &bindings()?.parameters, event.occurred_at_ms)
        else {
            return Ok(CreditOutcome::Refused);
        };
        changed |= after.phase_version != values[index].phase_version;
        values[index] = after;
    }
    sqlx::query("INSERT INTO game_world_change_credit_receipts(world_id,source_kind,occurrence,reset_epoch,binding_digest) VALUES(encode($1,'hex')::uuid,$2,$3,$4::text::numeric,$5)")
        .bind(world.as_bytes().as_slice()).bind(i16::from(event.source_kind)).bind(&event.occurrence).bind(authority.epoch.to_string()).bind(binding).execute(&mut **tx).await?;
    for &(kind, credit) in &event.credits {
        store(tx, world, kind, values[kind.index()]).await?;
        if matches!(kind, CounterKind::MagesTower | CounterKind::FireEarth)
            && credit == Credit::Kill(0)
            && (kind != CounterKind::FireEarth || values[kind.index()].stage == 1)
            && let Some(character) = character
        {
            let result:Option<String>=sqlx::query_scalar("INSERT INTO game_world_change_personal_counters(world_id,change_key,character_id,counter) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,1) ON CONFLICT(world_id,change_key,character_id) DO UPDATE SET counter=game_world_change_personal_counters.counter+1 RETURNING counter::text")
                .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
            let personal_count = result
                .ok_or(DurabilityError::InvalidStoredState)?
                .parse::<u64>()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if kind == CounterKind::FireEarth {
                fire_earth::observe_personal_credit(
                    tx,
                    character,
                    personal_count,
                    &event.occurrence,
                )
                .await?;
            }
        }
    }
    Ok(CreditOutcome::Applied {
        phase_changed: changed,
    })
}

impl DurabilityRoot {
    /// Native producer boundary: independent current scope/node facts are supplied by
    /// the owning runtime, never inferred from the retained death or quest receipt.
    #[allow(
        clippy::too_many_arguments,
        reason = "independent current authority and retained event are deliberately separate"
    )]
    pub async fn credit_world_change_event(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        epoch: u64,
        event: CreditEvent,
    ) -> Result<CreditOutcome, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let Some(current) =
                        authorize(&mut tx, &recovery, &node, scope, generation, epoch).await?
                    else {
                        return Ok(CreditOutcome::Refused);
                    };
                    let result = credit(&mut tx, &current, &event).await?;
                    if result != CreditOutcome::Refused {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(result)
                })
            })
            .await
    }
}

impl DurabilityRoot {
    /// Physical projections run under the same current World transaction. The synchronous
    /// callback may briefly try-lock the runtime; no runtime mutex spans a database await.
    #[allow(
        clippy::too_many_arguments,
        reason = "current authority and caller-owned physical context remain explicit"
    )]
    pub async fn project_world_change_counters<C: Send>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        epoch: u64,
        context: &mut C,
        project: fn(&mut C, &DailyWorldChanges) -> Result<(), DurabilityError>,
    ) -> Result<bool, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run_with_context(context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let Some(current) =
                        authorize(&mut tx, &recovery, &node, scope, generation, epoch).await?
                    else {
                        return Ok(false);
                    };
                    let values = states(&mut tx, scope.world_id(), true).await?;
                    let now: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let now =
                        u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)?;
                    if !current.current(&mut tx).await? {
                        return Ok(false);
                    }
                    for kind in CounterKind::ALL {
                        let before = values[kind.index()];
                        let after = before
                            .tick(kind, now)
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                        if after != before {
                            store(&mut tx, scope.world_id(), kind, after).await?;
                        }
                    }
                    let Some(day) = super::load(&mut tx, scope.world_id(), epoch).await? else {
                        return Ok(false);
                    };
                    project(context, &day)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(true)
                })
            })
            .await
    }
}

/// Observe the 25-use qualification only for the current shared encounter period.
/// The caller retains its existing Character/Channel authority; this read grants none.
pub async fn voice_eligible(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    character: CharacterId,
) -> Result<bool, DurabilityError> {
    if !super::load(tx, world, epoch).await?.is_some_and(|day| {
        oteryn_gameplay_world_change::owner::is_active(&day, "their_masters_voice")
    }) {
        return Ok(false);
    }
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_change_personal_counters p JOIN game_world_change_counters c USING(world_id,change_key) WHERE p.world_id=encode($1,'hex')::uuid AND p.change_key='their_masters_voice' AND p.character_id=encode($2,'hex')::uuid AND c.reset_epoch=$3::text::numeric AND p.counter>=25 AND p.period_deadline_ms=(c.state->>'deadline_ms')::bigint AND p.period_deadline_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint LIMIT 1)")
        .bind(world.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).bind(epoch.to_string())
        .fetch_one(&mut **tx).await.map_err(Into::into)
}
