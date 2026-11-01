//! BOSS-1 open-world clocks (§5). Scope ownership and node possession are independently
//! revalidated in the caller's semantic transaction. A successful CAS is pending durable state,
//! never authority to install an actor; install only after commit under the still-current owner.
//! Reset and death planners do not assert a current reset or a committed live-generation death.

use super::DurabilityError;
use super::character_authority::assert_recovery_fence;
use super::db::lock_admission_relations;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation, scope_key};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
#[cfg(test)]
use crate::domain::CharacterId;
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, BossClockError>;

#[derive(Debug)]
pub enum BossClockError {
    InvalidInput,
    InvalidStoredState,
    AuthorityRejected,
    MissingClock,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for BossClockError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<sqlx::Error> for BossClockError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
impl std::fmt::Display for BossClockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid boss clock input"),
            Self::InvalidStoredState => f.write_str("invalid boss clock state"),
            Self::AuthorityRejected => f.write_str("boss clock owner rejected"),
            Self::MissingClock => f.write_str("boss clock missing"),
            Self::Unavailable(error) => write!(f, "boss clock unavailable: {error:?}"),
        }
    }
}
impl std::error::Error for BossClockError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossClockKey {
    pub world: WorldId,
    pub channel: ChannelId,
    /// Opaque key of a qualified boss spawn in the current content, never a creature name.
    pub spawn: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossClockState {
    Due,
    Alive,
    Inactive,
}
impl BossClockState {
    fn stored(self) -> i16 {
        match self {
            Self::Due => 1,
            Self::Alive => 2,
            Self::Inactive => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossClock {
    pub cycle: u64,
    pub state: BossClockState,
    pub reset_epoch: u64,
    pub due_at_unix_ms: Option<i64>,
    pub owner_generation: Option<ScopeOwnershipGeneration>,
}
impl BossClock {
    fn validate(&self) -> Result<()> {
        let valid = match self.state {
            BossClockState::Due => {
                self.due_at_unix_ms.is_some_and(|time| time >= 0) && self.owner_generation.is_none()
            }
            BossClockState::Alive => {
                self.due_at_unix_ms.is_none() && self.owner_generation.is_some()
            }
            BossClockState::Inactive => {
                self.due_at_unix_ms.is_none() && self.owner_generation.is_none()
            }
        };
        if valid {
            Ok(())
        } else {
            Err(BossClockError::InvalidStoredState)
        }
    }
}

/// Historical/pending outcome only. No actor, live-generation or commit capability is minted.
#[derive(Debug, PartialEq, Eq)]
pub enum BossClockClaim {
    Claimed(BossClock),
    /// A committed clock snapshot used only to schedule the next owner timer.
    Waiting(BossClock),
    Unchanged,
    CompareFailed,
}

/// Claim/recover **respawn-policy** clocks only. Content must independently qualify this key as
/// a respawn boss. `at_reset` is deliberately not wired through this entry: its current epoch
/// must first be obtained from the actual WorldReset owner and durably drawn before realization.
///
/// The caller holds its real non-rollback recovery seal and runs this inside the native semantic
/// transaction/deadline. Admission relation locks precede scope/clock locks. Being the assigned
/// Channel owner suffices for activation before readiness publication; this grants no admission.
/// A missing clock is not initialized under a guessed first-spawn policy. The entire expected
/// row is CAS-bound, and a failed CAS emits no realization. Commit/install coupling is the native
/// owner caller's remaining integration obligation; an uncertain commit is not retried as spawn.
pub async fn claim_respawn_clock_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    key: &BossClockKey,
    generation: ScopeOwnershipGeneration,
    expected: &BossClock,
) -> Result<BossClockClaim> {
    if key.spawn.is_empty() {
        return Err(BossClockError::InvalidInput);
    }
    expected.validate()?;
    assert_clock_owner(tx, recovery, node, key, generation).await?;
    // DB wall-clock milliseconds are portable across owners; process-local owner timer instants
    // are deliberately not persisted. Resolve time after blocking authority locks.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let current = read_clock(tx, key)
        .await?
        .ok_or(BossClockError::MissingClock)?;
    if current != *expected {
        return Ok(BossClockClaim::CompareFailed);
    }
    let Some(next) = plan_claim(&current, generation, now)? else {
        return Ok(BossClockClaim::Unchanged);
    };
    if !compare_and_set_clock(tx, key, &current, &next).await? {
        return Ok(BossClockClaim::CompareFailed);
    }
    Ok(BossClockClaim::Claimed(next))
}

async fn assert_clock_owner(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    key: &BossClockKey,
    generation: ScopeOwnershipGeneration,
) -> Result<()> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    // Native assignment reads reject a materialized row not backed by the retained writer
    // high-water/history. The native function retains the writer's share lock in this TX.
    let history_valid: bool =
        sqlx::query_scalar("SELECT game_runtime_scope_assignment_history_valid()")
            .fetch_one(&mut **tx)
            .await?;
    if !history_valid {
        return Err(BossClockError::InvalidStoredState);
    }
    let fact = node.fact();
    let assigned = sqlx::query(
        "SELECT 1 FROM game_runtime_scope_assignments \
          WHERE scope_key=$1 AND scope_kind=1 AND state=1 \
            AND world_id=encode($2,'hex')::uuid AND channel_id=encode($3,'hex')::uuid \
            AND ownership_generation=$4::text::numeric(20,0) \
            AND holder_node_id=encode($5,'hex')::uuid \
            AND holder_registration_revision=$6::text::numeric(20,0) FOR SHARE",
    )
    .bind(scope_key(key.world, key.channel).as_slice())
    .bind(key.world.as_bytes().as_slice())
    .bind(key.channel.as_bytes().as_slice())
    .bind(generation.get().to_string())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_optional(&mut **tx)
    .await?
    .is_some();
    if !assigned || !prove_current_incarnation(tx, node).await? {
        return Err(BossClockError::AuthorityRejected);
    }
    Ok(())
}

pub async fn read_clock(
    tx: &mut Transaction<'_, Postgres>,
    key: &BossClockKey,
) -> Result<Option<BossClock>> {
    let row = sqlx::query("SELECT cycle::text,state,reset_epoch::text,due_at,owner_generation::text FROM game_boss_spawn_clocks WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND spawn_key=$3")
        .bind(key.world.as_bytes().as_slice()).bind(key.channel.as_bytes().as_slice()).bind(&key.spawn).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        let invalid = || BossClockError::InvalidStoredState;
        let state = match row.try_get::<i16, _>("state")? {
            1 => BossClockState::Due,
            2 => BossClockState::Alive,
            3 => BossClockState::Inactive,
            _ => return Err(invalid()),
        };
        let owner: Option<String> = row.try_get("owner_generation")?;
        let clock = BossClock {
            cycle: row
                .try_get::<String, _>("cycle")?
                .parse()
                .map_err(|_| invalid())?,
            state,
            reset_epoch: row
                .try_get::<String, _>("reset_epoch")?
                .parse()
                .map_err(|_| invalid())?,
            due_at_unix_ms: row.try_get("due_at")?,
            owner_generation: owner
                .map(|value| {
                    value
                        .parse::<u64>()
                        .map_err(|_| invalid())
                        .and_then(|value| {
                            ScopeOwnershipGeneration::new(value).map_err(|_| invalid())
                        })
                })
                .transpose()?,
        };
        clock.validate()?;
        Ok(clock)
    })
    .transpose()
}

// Private primitive, not an alternative public write path. The only production caller above
// independently proves current authority and permits only claim/recovery, not reset/death.
async fn compare_and_set_clock(
    tx: &mut Transaction<'_, Postgres>,
    key: &BossClockKey,
    before: &BossClock,
    after: &BossClock,
) -> Result<bool> {
    before.validate()?;
    after.validate()?;
    Ok(sqlx::query("UPDATE game_boss_spawn_clocks SET cycle=$4::text::numeric(20,0),state=$5,reset_epoch=$6::text::numeric(20,0),due_at=$7,owner_generation=$8::text::numeric(20,0) WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND spawn_key=$3 AND cycle=$9::text::numeric(20,0) AND state=$10 AND reset_epoch=$11::text::numeric(20,0) AND due_at IS NOT DISTINCT FROM $12 AND owner_generation IS NOT DISTINCT FROM $13::text::numeric(20,0)")
        .bind(key.world.as_bytes().as_slice()).bind(key.channel.as_bytes().as_slice()).bind(&key.spawn)
        .bind(after.cycle.to_string()).bind(after.state.stored()).bind(after.reset_epoch.to_string()).bind(after.due_at_unix_ms).bind(after.owner_generation.map(|value|value.get().to_string()))
        .bind(before.cycle.to_string()).bind(before.state.stored()).bind(before.reset_epoch.to_string()).bind(before.due_at_unix_ms).bind(before.owner_generation.map(|value|value.get().to_string()))
        .execute(&mut **tx).await?.rows_affected()==1)
}

fn plan_claim(
    clock: &BossClock,
    generation: ScopeOwnershipGeneration,
    now_unix_ms: i64,
) -> Result<Option<BossClock>> {
    clock.validate()?;
    if now_unix_ms < 0 {
        return Err(BossClockError::InvalidInput);
    }
    let claim = match clock.state {
        BossClockState::Due => clock.due_at_unix_ms.is_some_and(|due| due <= now_unix_ms),
        BossClockState::Inactive => false,
        BossClockState::Alive => {
            let owner = clock
                .owner_generation
                .ok_or(BossClockError::InvalidStoredState)?;
            if owner > generation {
                return Err(BossClockError::AuthorityRejected);
            }
            owner < generation
        }
    };
    Ok(claim.then(|| BossClock {
        state: BossClockState::Alive,
        due_at_unix_ms: None,
        owner_generation: Some(generation),
        ..clock.clone()
    }))
}

/// Pure proposal, not evidence of a committed death. The death owner must persist it in the
/// same live-generation death descendant transaction. Draw identity is the **new** cycle.
pub fn plan_respawn_death(
    key: &BossClockKey,
    clock: &BossClock,
    death_unix_ms: i64,
    min_s: u64,
    max_s: u64,
) -> Result<BossClock> {
    clock.validate()?;
    if clock.state != BossClockState::Alive || death_unix_ms < 0 || min_s == 0 || min_s > max_s {
        return Err(BossClockError::InvalidInput);
    }
    let cycle = clock
        .cycle
        .checked_add(1)
        .ok_or(BossClockError::InvalidStoredState)?;
    let minimum = i64::try_from(min_s).map_err(|_| BossClockError::InvalidInput)?;
    let maximum = i64::try_from(max_s).map_err(|_| BossClockError::InvalidInput)?;
    let draw = draw(key, cycle, "boss_respawn")?;
    let seconds =
        oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(draw, minimum, maximum);
    let due = seconds
        .checked_mul(1000)
        .and_then(|delta| death_unix_ms.checked_add(delta))
        .ok_or(BossClockError::InvalidInput)?;
    Ok(BossClock {
        cycle,
        state: BossClockState::Due,
        due_at_unix_ms: Some(due),
        owner_generation: None,
        ..clock.clone()
    })
}

/// Pure at-reset proposal: called before claim/recovery, never after realizing an old ALIVE
/// row. The actual reset owner must independently supply the current epoch/time. Chance is an
/// exact content ratio; no engine percentage/ppm unit or global default is introduced.
pub fn plan_reset(
    key: &BossClockKey,
    clock: &BossClock,
    epoch: u64,
    reset_unix_ms: i64,
    numerator: u64,
    denominator: u64,
) -> Result<Option<BossClock>> {
    clock.validate()?;
    if epoch < clock.reset_epoch
        || reset_unix_ms < 0
        || denominator == 0
        || numerator > denominator
        || denominator > i64::MAX as u64
    {
        return Err(BossClockError::InvalidInput);
    }
    if epoch == clock.reset_epoch {
        return Ok(None);
    }
    let draw = draw(key, epoch, "boss_at_reset")?;
    let yes = oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
        draw,
        0,
        (denominator - 1) as i64,
    ) < numerator as i64;
    Ok(Some(BossClock {
        state: if yes {
            BossClockState::Due
        } else {
            BossClockState::Inactive
        },
        reset_epoch: epoch,
        due_at_unix_ms: yes.then_some(reset_unix_ms),
        owner_generation: None,
        ..clock.clone()
    }))
}

pub fn plan_reset_death(clock: &BossClock) -> Result<BossClock> {
    clock.validate()?;
    if clock.state != BossClockState::Alive {
        return Err(BossClockError::InvalidInput);
    }
    Ok(BossClock {
        cycle: clock
            .cycle
            .checked_add(1)
            .ok_or(BossClockError::InvalidStoredState)?,
        state: BossClockState::Inactive,
        due_at_unix_ms: None,
        owner_generation: None,
        ..clock.clone()
    })
}

// Versioned unambiguous coordinate encoding only; the sole generator is native deterministic
// decision RNG. World and Channel have distinct fixed fields; spawn text has an explicit length.
fn draw(key: &BossClockKey, ordinal: u64, purpose: &str) -> Result<u64> {
    if key.spawn.is_empty() {
        return Err(BossClockError::InvalidInput);
    }
    let mut seed = Sha256::new();
    seed.update(b"oteryn:boss-clock:v1");
    seed.update(key.world.as_bytes());
    seed.update(key.channel.as_bytes());
    seed.update((key.spawn.len() as u64).to_be_bytes());
    seed.update(key.spawn.as_bytes());
    let root = GameplayDecisionRoot::from_bytes(seed.finalize().into());
    let mut occurrence = [0; 16];
    occurrence[8..].copy_from_slice(&ordinal.to_be_bytes());
    deterministic_decision_u64(
        &root,
        DecisionOccurrenceId::from_bytes(occurrence),
        purpose,
        0,
    )
    .map_err(|_| BossClockError::InvalidInput)
}

#[cfg(test)]
#[path = "boss_encounter/activity_assignment_tests.rs"]
mod activity_assignment_tests;

#[path = "boss_encounter/cooldown_storage.rs"]
pub(crate) mod cooldown_storage;

#[path = "boss_encounter/cooldown_list.rs"]
mod cooldown_list;
#[path = "boss_encounter/reward_chest.rs"]
pub mod reward_chest;

// These private preparation primitives had no production caller in the source file.
// Keep qualifying them without exposing an unfenced admission/death write API.
#[cfg(test)]
#[path = "boss_encounter/preparation.rs"]
mod preparation;

impl super::DurabilityRoot {
    /// One activation/timer pass. The current reset pointer and ownership are read under
    /// their database locks, never supplied by content or an old process snapshot.
    pub async fn activate_boss_clock(
        &self,
        authority: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        key: BossClockKey,
        generation: ScopeOwnershipGeneration,
        policy: oteryn_gameplay_content::content::BossPolicy,
    ) -> Result<BossClockClaim> {
        policy
            .validate()
            .map_err(|_| BossClockError::InvalidInput)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| BossClockError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            use super::db::{begin_semantic_transaction, commit_semantic_transaction};
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            // Keep domain rejection inside the pass's DurabilityError envelope.
            let result: Result<BossClockClaim> = async {
                assert_clock_owner(&mut tx, &recovery, &node, &key, generation).await?;
                let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
                let reset: Option<(String,i64)> = sqlx::query_as("SELECT a.reset_epoch::text, floor(extract(epoch FROM e.activated_at)*1000)::bigint FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid FOR SHARE OF a,e")
                    .bind(key.world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                let reset_initialized = reset.is_some();
                let (epoch, reset_time) = match reset {
                    Some((epoch,time)) => (epoch.parse().map_err(|_| BossClockError::InvalidStoredState)?, time),
                    None => (0, now),
                };
                if matches!(policy, oteryn_gameplay_content::content::BossPolicy::AtReset { .. }) && !reset_initialized {
                    // No actual reset owner yet: no invented reset or draw.
                    return Ok(BossClockClaim::Unchanged);
                }
                let mut initial = BossClock { cycle: 0, state: BossClockState::Inactive,
                    reset_epoch: 0, due_at_unix_ms: None, owner_generation: None };
                if let oteryn_gameplay_content::content::BossPolicy::Respawn { min_s, max_s } = policy {
                    let seconds = oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
                        draw(&key,0,"boss_respawn")?, min_s as i64, max_s as i64);
                    initial.state = BossClockState::Due;
                    initial.due_at_unix_ms = Some(seconds.checked_mul(1000).and_then(|d| now.checked_add(d)).ok_or(BossClockError::InvalidInput)?);
                    initial.reset_epoch = epoch;
                }
                if let oteryn_gameplay_content::content::BossPolicy::AtReset { chance } = policy {
                    let yes = oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
                        draw(&key,epoch,"boss_at_reset")?,0,(chance.denominator-1) as i64) < chance.numerator as i64;
                    initial.reset_epoch = epoch;
                    if yes { initial.state = BossClockState::Due; initial.due_at_unix_ms = Some(reset_time); }
                }
                sqlx::query("INSERT INTO game_boss_spawn_clocks(world_id,channel_id,spawn_key,cycle,state,reset_epoch,due_at,owner_generation) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,0,$4,$5::text::numeric,$6,NULL) ON CONFLICT DO NOTHING")
                    .bind(key.world.as_bytes().as_slice()).bind(key.channel.as_bytes().as_slice()).bind(&key.spawn)
                    .bind(initial.state.stored()).bind(initial.reset_epoch.to_string()).bind(initial.due_at_unix_ms).execute(&mut *tx).await?;
                let mut clock = read_clock(&mut tx, &key).await?.ok_or(BossClockError::MissingClock)?;
                if let oteryn_gameplay_content::content::BossPolicy::AtReset { chance } = policy {
                    if let Some(next) = plan_reset(&key,&clock,epoch,reset_time,chance.numerator,chance.denominator)? {
                        if !compare_and_set_clock(&mut tx,&key,&clock,&next).await? { return Ok(BossClockClaim::CompareFailed); }
                        clock = next;
                    }
                    if let Some(next) = plan_claim(&clock,generation,now)? {
                        if !compare_and_set_clock(&mut tx,&key,&clock,&next).await? { return Ok(BossClockClaim::CompareFailed); }
                        return Ok(BossClockClaim::Claimed(next));
                    }
                    Ok(BossClockClaim::Unchanged)
                } else {
                    claim_respawn_clock_in_transaction(&mut tx,&recovery,&node,&key,generation,&clock).await
                }
            }.await;
            let result = match result {
                Ok(BossClockClaim::Unchanged) => {
                    match read_clock(&mut tx,&key).await.map_err(|_| super::DurabilityError::InvalidStoredState)? {
                        Some(clock) if clock.state != BossClockState::Alive => Ok(BossClockClaim::Waiting(clock)),
                        _ => Ok(BossClockClaim::Unchanged),
                    }
                }
                other => other,
            };
            if result.is_ok() { commit_semantic_transaction(tx,deadline).await?; }
            Ok(result)
        })).await?
    }
}

/// Called only after the corpse receipt's replay check and current-generation check.
/// A failure rolls the corpse and clock back together; replay returns before this call.
pub(crate) async fn advance_death_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    key: &BossClockKey,
    generation: ScopeOwnershipGeneration,
    cycle: u64,
    policy: oteryn_gameplay_content::content::BossPolicy,
) -> Result<()> {
    policy
        .validate()
        .map_err(|_| BossClockError::InvalidInput)?;
    let clock = read_clock(tx, key)
        .await?
        .ok_or(BossClockError::MissingClock)?;
    if clock.state != BossClockState::Alive
        || clock.owner_generation != Some(generation)
        || clock.cycle != cycle
    {
        return Err(BossClockError::AuthorityRejected);
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let next = match policy {
        oteryn_gameplay_content::content::BossPolicy::Respawn { min_s, max_s } => {
            plan_respawn_death(key, &clock, now, min_s, max_s)?
        }
        oteryn_gameplay_content::content::BossPolicy::AtReset { .. } => plan_reset_death(&clock)?,
    };
    if !compare_and_set_clock(tx, key, &clock, &next).await? {
        return Err(BossClockError::AuthorityRejected);
    }
    Ok(())
}
