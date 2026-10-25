//! Metadata CAS only; caller owns its semantic transaction/deadline and commit.
//! Recovery and admission locks precede the named World/raid lock. Proposed
//! reset/content facts do not authorize a World reset, actor or announcement.
mod storage;

use super::{
    DurabilityError, character_authority::assert_recovery_fence, db::lock_admission_relations,
};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::content::ProductionKey;
use crate::foundation::WorldId;
use oteryn_gameplay_encounter_rules::raid::{RaidError, RaidSchedule, RaidTransition};
use sqlx::{Postgres, Transaction};

#[derive(Debug)]
pub enum RaidStoreError {
    Rules(RaidError),
    InvalidStoredState,
    Unavailable(DurabilityError),
}
impl std::fmt::Display for RaidStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "raid metadata store: {self:?}")
    }
}
impl std::error::Error for RaidStoreError {}
impl From<RaidError> for RaidStoreError {
    fn from(error: RaidError) -> Self {
        Self::Rules(error)
    }
}
impl From<sqlx::Error> for RaidStoreError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
impl From<DurabilityError> for RaidStoreError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

/// Pending metadata disposition; none of these outcomes is a commit token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidWrite {
    Applied,
    Replayed,
    CompareFailed,
}

pub use storage::load;

pub async fn initialize(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    world: WorldId,
    key: &ProductionKey,
    reset_epoch: u64,
    now_ms: i64,
) -> Result<RaidWrite, RaidStoreError> {
    let initial = RaidSchedule::initial(reset_epoch, now_ms)?;
    lock(tx, recovery, world, key).await?;
    if let Some(current) = load(tx, world, key).await? {
        return Ok(if current == initial {
            RaidWrite::Replayed
        } else {
            RaidWrite::CompareFailed
        });
    }
    storage::insert_schedule(tx, world, key, &initial).await?;
    Ok(RaidWrite::Applied)
}

pub async fn compare_and_set(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    transition: &RaidTransition,
) -> Result<RaidWrite, RaidStoreError> {
    transition.before().validate()?;
    transition.after().validate()?;
    let world = transition.world();
    let key = transition.key();
    lock(tx, recovery, world, key).await?;
    let Some(current) = load(tx, world, key).await? else {
        return Ok(RaidWrite::CompareFailed);
    };
    // Whole snapshot equality includes the firing status, digest and ID, not
    // merely a revision or ordinal. Same-head retries are strictly idempotent.
    if current == *transition.after() {
        return Ok(RaidWrite::Replayed);
    }
    if current != *transition.before() {
        return Ok(RaidWrite::CompareFailed);
    }
    storage::write_transition(tx, world, key, transition).await?;
    Ok(RaidWrite::Applied)
}

async fn lock(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    world: WorldId,
    key: &ProductionKey,
) -> Result<(), RaidStoreError> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    // UUID is fixed width; the remaining key is bounded by ProductionKey.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:raid:' || encode($1,'hex') || ':' || $2,0))")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).execute(&mut **tx).await?;
    Ok(())
}
