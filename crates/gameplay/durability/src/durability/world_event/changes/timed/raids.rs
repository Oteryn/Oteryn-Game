//! A native actor's source is the immutable admitted firing, not its display name.
use super::*;
use crate::domain::CharacterId;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_entities::carrier::RaidSpawnIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaidOrigin {
    pub kind: EventKind,
    pub cycle: i64,
    pub generation: u64,
    pub member: u16,
    pub version: u64,
}
impl RaidOrigin {
    pub(crate) fn from_state(
        kind: EventKind,
        epoch: u64,
        member: u16,
        state: &EventState,
    ) -> Result<Self, DurabilityError> {
        Ok(Self {
            kind,
            cycle: cycle(kind, epoch, state.start_ms)?,
            generation: state.lifecycle.generation,
            member,
            version: if matches!(kind, EventKind::SpringIntoLife | EventKind::BankRobbery) {
                state
                    .lifecycle
                    .objects
                    .get(&member)
                    .map_or(u64::from(state.wave), |o| o.revision())
            } else {
                u64::from(state.wave)
            },
        })
    }
    fn valid(&self) -> bool {
        self.cycle >= 0
            && match self.kind {
                EventKind::SpringIntoLife => (1..=2).contains(&self.member),
                EventKind::RiseOfDevovorga => self.member <= 8,
                EventKind::DevovorgasEssence => self.member <= 4,
                _ => self.member == 0,
            }
    }
}

pub(crate) async fn raid_origin(
    tx: &mut Transaction<'_, Postgres>,
    firing: [u8; 16],
) -> Result<Option<RaidOrigin>, DurabilityError> {
    let raw: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT origin FROM game_world_timed_raid_origins WHERE firing_id=encode($1,'hex')::uuid",
    )
    .bind(firing.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let origin: Option<RaidOrigin> = raw
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if origin.as_ref().is_some_and(|o| !o.valid()) {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(origin)
}

pub(crate) async fn seal_raid_origin(
    tx: &mut Transaction<'_, Postgres>,
    firing: [u8; 16],
    origin: &RaidOrigin,
) -> Result<(), DurabilityError> {
    if !origin.valid() {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("INSERT INTO game_world_timed_raid_origins VALUES(encode($1,'hex')::uuid,$2)")
        .bind(firing.as_slice())
        .bind(serde_json::to_value(origin).map_err(|_| DurabilityError::InvalidStoredState)?)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Frozen by the lethal native producer. No transport constructs this input.
#[derive(Debug, Clone)]
pub struct NativeKillRequest {
    pub source: RaidSpawnIdentity,
    pub occurrence: Vec<u8>,
    pub occurred_at_ms: i64,
    pub participants: Vec<CharacterId>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeKillOutcome {
    Pending,
    Applied,
    AlreadyApplied,
    Refused,
}

async fn credit(
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    epoch: u64,
    event: &NativeKillRequest,
    clock: Option<i64>,
) -> Result<NativeKillOutcome, DurabilityError> {
    let occurred = clock.unwrap_or(event.occurred_at_ms);
    let RuntimeScopeRefV1::Channel {
        world_id: world,
        channel_id: channel,
    } = scope
    else {
        return Ok(NativeKillOutcome::Refused);
    };
    let mut people = event.participants.clone();
    people.sort_by_key(|id| *id.as_bytes());
    people.dedup();
    // The native damage ledger retains sixteen contributors plus the proven
    // lethal principal when that bounded ledger could not retain another entry.
    if people.is_empty()
        || people.len() > 17
        || people.len() != event.participants.len()
        || event.occurred_at_ms < 0
        || occurred < 0
        || event.occurrence.len() != 52
        || event.occurrence[..16] != world.as_bytes()[..]
        || event.occurrence[16..32] != channel.as_bytes()[..]
        || event.occurrence[32..40] != generation.get().to_be_bytes()
    {
        return Ok(NativeKillOutcome::Refused);
    }
    let source = event.source;
    let (Ok(wave), Ok(entry), Ok(index)) = (
        i16::try_from(source.wave),
        i16::try_from(source.entry),
        i16::try_from(source.index),
    ) else {
        return Ok(NativeKillOutcome::Refused);
    };
    let admitted: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_firings f JOIN game_world_raid_targets t USING(firing_id) JOIN game_world_raid_creature_placements p USING(firing_id,channel_id) WHERE f.firing_id=encode($1,'hex')::uuid AND f.world_id=encode($2,'hex')::uuid AND f.reset_epoch=$3::text::numeric AND f.state IN(1,2) AND NOT EXISTS(SELECT 1 FROM game_world_raid_retirements r WHERE r.firing_id=f.firing_id) AND t.channel_id=encode($4,'hex')::uuid AND t.owner_generation=$5::text::numeric AND p.wave=$6 AND p.entry=$7 AND p.creature_index=$8 AND p.placed)")
        .bind(source.firing_id.as_slice()).bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(channel.as_bytes().as_slice()).bind(generation.get().to_string()).bind(wave).bind(entry).bind(index).fetch_one(&mut **tx).await?;
    if !admitted {
        // A live actor can die before its placement ACK reaches PostgreSQL.
        // Keep the native kill queued until that independently fenced ACK lands.
        let awaiting: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_firings f JOIN game_world_timed_raid_origins o USING(firing_id) JOIN game_world_raid_targets t USING(firing_id) JOIN game_world_raid_creatures c USING(firing_id,channel_id) LEFT JOIN game_world_raid_creature_placements p USING(firing_id,channel_id,wave,entry,creature_index) WHERE f.firing_id=encode($1,'hex')::uuid AND f.world_id=encode($2,'hex')::uuid AND f.reset_epoch=$3::text::numeric AND f.state IN(1,2) AND NOT EXISTS(SELECT 1 FROM game_world_raid_retirements r WHERE r.firing_id=f.firing_id) AND t.channel_id=encode($4,'hex')::uuid AND t.owner_generation=$5::text::numeric AND c.wave=$6 AND c.entry=$7 AND c.creature_index=$8 AND p.firing_id IS NULL)")
            .bind(source.firing_id.as_slice()).bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(channel.as_bytes().as_slice()).bind(generation.get().to_string()).bind(wave).bind(entry).bind(index).fetch_one(&mut **tx).await?;
        return Ok(if awaiting {
            NativeKillOutcome::Pending
        } else {
            NativeKillOutcome::Refused
        });
    }
    let Some(origin) = raid_origin(tx, source.firing_id).await? else {
        return Ok(NativeKillOutcome::Refused);
    };
    let Some(bounds) = window(tx, world, epoch, origin.kind, occurred).await? else {
        return Ok(NativeKillOutcome::Refused);
    };
    if occurred >= bounds.1 || cycle(origin.kind, epoch, bounds.0)? != origin.cycle {
        return Ok(NativeKillOutcome::Refused);
    }
    // Quest transactions own the Character root before the World cycle. Taking
    // these FK locks first preserves that order for a multi-player native death.
    for person in &people {
        if sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR KEY SHARE")
            .bind(person.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.is_none() {return Ok(NativeKillOutcome::Refused);}
    }
    let mut state = state(tx, world, origin.kind, bounds, origin.cycle, occurred).await?;
    let mut hash = Sha256::new();
    hash.update(b"oteryn:wevents:native-credit:v1\0");
    hash.update(source.firing_id);
    hash.update(source.wave.to_be_bytes());
    hash.update(source.entry.to_be_bytes());
    hash.update(source.index.to_be_bytes());
    hash.update(
        u64::try_from(people.len())
            .map_err(|_| DurabilityError::InvalidStoredState)?
            .to_be_bytes(),
    );
    for person in &people {
        hash.update(person.as_bytes());
    }
    hash.update(event.occurred_at_ms.to_be_bytes());
    hash.update(serde_json::to_vec(&origin).map_err(|_| DurabilityError::InvalidStoredState)?);
    let binding = hash.finalize().to_vec();
    let previous: Option<Vec<u8>>=sqlx::query_scalar("SELECT input_binding FROM game_world_timed_kill_receipts WHERE world_id=encode($1,'hex')::uuid AND event_key=$2 AND cycle_id=$3 AND occurrence=$4")
        .bind(world.as_bytes().as_slice()).bind(origin.kind.key()).bind(origin.cycle).bind(&event.occurrence).fetch_optional(&mut **tx).await?;
    if let Some(previous) = previous {
        return Ok(if previous == binding {
            NativeKillOutcome::AlreadyApplied
        } else {
            NativeKillOutcome::Refused
        });
    }
    let used_source: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_timed_kill_receipts WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND wave=$3 AND entry=$4 AND creature_index=$5)")
        .bind(source.firing_id.as_slice()).bind(channel.as_bytes().as_slice()).bind(wave).bind(entry).bind(index).fetch_one(&mut **tx).await?;
    if used_source {
        return Ok(NativeKillOutcome::Refused);
    }
    if origin.kind == EventKind::RiseOfDevovorga {
        let now = occurred.max(state.observed_ms);
        state
            .advance(origin.kind, now, Input::Tick, 0)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        // A late death may expire the shared attempt, but cannot win it.
        save(tx, world, origin.kind, &state, origin.cycle).await?;
    }
    if (matches!(
        origin.kind,
        EventKind::RiseOfDevovorga | EventKind::BankRobbery | EventKind::InsectoidInvasion
    ) && state.phase != oteryn_gameplay_world_change::timed::Phase::Active)
        || state.lifecycle.generation != origin.generation
        || !matches!(
            state.phase,
            oteryn_gameplay_world_change::timed::Phase::Active
                | oteryn_gameplay_world_change::timed::Phase::Victory
        )
    {
        return Ok(NativeKillOutcome::Refused);
    }
    let final_credit = origin.kind == EventKind::RiseOfDevovorga
        && origin.member == 5
        && state.lifecycle.stage == 5;
    if matches!(
        origin.kind,
        EventKind::RiseOfDevovorga | EventKind::Kingsday
    ) {
        let now = occurred.max(state.observed_ms);
        state
            .advance(
                origin.kind,
                now,
                Input::Defeat(
                    u8::try_from(origin.member).map_err(|_| DurabilityError::InvalidStoredState)?,
                ),
                0,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    for person in &people {
        sqlx::query("INSERT INTO game_world_timed_participants(world_id,event_key,cycle_id,character_id,credited,reward_ready) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,1,$5) ON CONFLICT(world_id,event_key,cycle_id,character_id) DO UPDATE SET credited=game_world_timed_participants.credited+1,reward_ready=game_world_timed_participants.reward_ready OR $5")
            .bind(world.as_bytes().as_slice()).bind(origin.kind.key()).bind(origin.cycle).bind(person.as_bytes().as_slice()).bind(final_credit).execute(&mut **tx).await?;
    }
    save(tx, world, origin.kind, &state, origin.cycle).await?;
    sqlx::query("INSERT INTO game_world_timed_kill_receipts VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,$10)")
        .bind(world.as_bytes().as_slice()).bind(origin.kind.key()).bind(origin.cycle).bind(&event.occurrence).bind(binding).bind(source.firing_id.as_slice()).bind(channel.as_bytes().as_slice()).bind(wave).bind(entry).bind(index).execute(&mut **tx).await?;
    Ok(NativeKillOutcome::Applied)
}

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "current authority is independent from the frozen native source"
    )]
    pub async fn credit_timed_world_kill(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        epoch: u64,
        event: NativeKillRequest,
    ) -> Result<NativeKillOutcome, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let clock = self.world_event_clock_override_ms();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    if super::super::counters::authorize(
                        &mut tx, &recovery, &node, scope, generation, epoch,
                    )
                    .await?
                    .is_none()
                    {
                        return Ok(NativeKillOutcome::Refused);
                    }
                    let result = credit(&mut tx, scope, generation, epoch, &event, clock).await?;
                    // Calendar/attempt expiry is World state even when this
                    // independently current native source no longer earns credit.
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
}
