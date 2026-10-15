//! BOSS-1 open-world clocks (§5). Scope ownership and node possession are independently
//! revalidated in the caller's semantic transaction. A successful CAS is pending durable state,
//! never authority to install an actor; install only after commit under the still-current owner.
//! Reset and death planners do not assert a current reset or a committed live-generation death.

use super::DurabilityError;
use super::character_authority::assert_recovery_fence;
use super::db::lock_admission_relations;
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation, scope_key};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, BossClockError>;

#[derive(Debug)]
pub(crate) enum BossClockError {
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
pub(crate) struct BossClockKey {
    pub(crate) world: WorldId,
    pub(crate) channel: ChannelId,
    /// Opaque key of a qualified boss spawn in the current content, never a creature name.
    pub(crate) spawn: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BossClockState {
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
pub(crate) struct BossClock {
    pub(crate) cycle: u64,
    pub(crate) state: BossClockState,
    pub(crate) reset_epoch: u64,
    pub(crate) due_at_unix_ms: Option<i64>,
    pub(crate) owner_generation: Option<ScopeOwnershipGeneration>,
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
pub(crate) enum BossClockClaim {
    Claimed(BossClock),
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
pub(crate) async fn claim_respawn_clock_in_transaction(
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

pub(crate) async fn read_clock(
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
pub(crate) fn plan_respawn_death(
    key: &BossClockKey,
    clock: &BossClock,
    death_unix_ms: i64,
    min_s: u64,
    max_s: u64,
) -> Result<BossClock> {
    clock.validate()?;
    if clock.state != BossClockState::Alive || death_unix_ms < 0 || min_s > max_s {
        return Err(BossClockError::InvalidInput);
    }
    let cycle = clock
        .cycle
        .checked_add(1)
        .ok_or(BossClockError::InvalidStoredState)?;
    let minimum = i64::try_from(min_s).map_err(|_| BossClockError::InvalidInput)?;
    let maximum = i64::try_from(max_s).map_err(|_| BossClockError::InvalidInput)?;
    let draw = draw(key, cycle, "boss_respawn")?;
    let seconds = crate::spell::uniform_draw(draw, minimum, maximum);
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
pub(crate) fn plan_reset(
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
    let yes = crate::spell::uniform_draw(draw, 0, (denominator - 1) as i64) < numerator as i64;
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

pub(crate) fn plan_reset_death(clock: &BossClock) -> Result<BossClock> {
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
#[allow(clippy::expect_used)]
#[path = "boss_encounter_tests.rs"]
mod tests;
