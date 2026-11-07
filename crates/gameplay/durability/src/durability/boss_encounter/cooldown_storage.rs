//! Shared Character boss cooldown storage, used only inside a fully fenced
//! activity admission transaction. Extracted from the existing preparation owner;
//! Character-order locks, full-group preflight and exact admission replay remain.
use super::super::DurabilityError;
use crate::content::ProductionKey;
use crate::domain::CharacterId;
use sqlx::{Postgres, Row, Transaction};

pub(crate) type RewardResult<T> = std::result::Result<T, BossRewardStateError>;

#[derive(Debug)]
pub(crate) enum BossRewardStateError {
    InvalidInput,
    InvalidStoredState,
    #[cfg(test)]
    AuthorityRejected,
    ConflictingAdmission,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for BossRewardStateError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<sqlx::Error> for BossRewardStateError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
impl std::fmt::Display for BossRewardStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid boss reward state input"),
            Self::InvalidStoredState => f.write_str("invalid boss reward stored state"),
            #[cfg(test)]
            Self::AuthorityRejected => f.write_str("boss contributor authority rejected"),
            Self::ConflictingAdmission => f.write_str("boss admission reused with different state"),
            Self::Unavailable(error) => write!(f, "boss reward state unavailable: {error:?}"),
        }
    }
}
impl std::error::Error for BossRewardStateError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BossRewardKey {
    pub(crate) character: CharacterId,
    pub(crate) boss: ProductionKey,
}
impl BossRewardKey {
    pub(crate) fn new(character: CharacterId, boss: &str) -> RewardResult<Self> {
        Ok(Self {
            character,
            boss: ProductionKey::new(boss).map_err(|_| BossRewardStateError::InvalidInput)?,
        })
    }
}

pub(crate) fn reward_deadline(now: i64, seconds: u64) -> RewardResult<i64> {
    if now < 0 {
        return Err(BossRewardStateError::InvalidInput);
    }
    i64::try_from(seconds)
        .ok()
        .and_then(|seconds| seconds.checked_mul(1000))
        .and_then(|delta| now.checked_add(delta))
        .ok_or(BossRewardStateError::InvalidInput)
}

pub(crate) fn reward_uuid(value: &[u8; 16]) -> bool {
    value[6] >> 4 == 7 && value[8] & 0xc0 == 0x80
}

// Character roots serialize missing singleton/cooldown rows too. Sorting distinct roots
// before rows preserves §12 even for a group. This is only a lock, never an authority check.
pub(crate) async fn lock_reward_roots(
    tx: &mut Transaction<'_, Postgres>,
    keys: &[BossRewardKey],
) -> RewardResult<()> {
    let characters: std::collections::BTreeSet<_> = keys.iter().map(|key| key.character).collect();
    for character in characters {
        let exists = sqlx::query(
            "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
        )
        .bind(character.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await?
        .is_some();
        if !exists {
            return Err(BossRewardStateError::InvalidStoredState);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BossCooldownData {
    pub(crate) available_at: i64,
    pub(crate) admission: [u8; 16],
    pub(crate) reset_epoch: u64,
}

pub(crate) async fn read_boss_cooldown_data(
    tx: &mut Transaction<'_, Postgres>,
    key: &BossRewardKey,
) -> RewardResult<Option<BossCooldownData>> {
    let row = sqlx::query(
        "SELECT available_at,uuid_send(admission) AS admission_bytes,reset_epoch::text \
         FROM game_character_boss_cooldowns \
         WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 FOR UPDATE",
    )
    .bind(key.character.as_bytes().as_slice())
    .bind(key.boss.as_str())
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|row| {
        let admission: Vec<u8> = row.try_get("admission_bytes")?;
        let value = BossCooldownData {
            available_at: row.try_get("available_at")?,
            admission: admission
                .try_into()
                .map_err(|_| BossRewardStateError::InvalidStoredState)?,
            reset_epoch: row
                .try_get::<String, _>("reset_epoch")?
                .parse()
                .map_err(|_| BossRewardStateError::InvalidStoredState)?,
        };
        if value.available_at < 0 || !reward_uuid(&value.admission) {
            return Err(BossRewardStateError::InvalidStoredState);
        }
        Ok(value)
    })
    .transpose()
}

/// Proposed facts ONLY. Current epoch, boosted-boss identity, lever predicates and the
/// admission occurrence must be resolved by BOSS-ROOM-1 before these data helpers are wired.
pub(crate) struct BossCooldownAdmissionData {
    pub(crate) admission: [u8; 16],
    pub(crate) now: i64,
    pub(crate) reset_epoch: u64,
    pub(crate) cooldown_s: u64,
    pub(crate) boosted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BossCooldownChange {
    Applied,
    Replayed,
    OnCooldown,
}

// No partial group write on a refusal: validate and lock every root/row in deterministic
// order, then preflight the entire group before the first write. Allocation ADMITTING and
// cooldowns still must commit together in the real lever owner, which this does not replace.
pub(crate) async fn set_boss_cooldowns_data(
    tx: &mut Transaction<'_, Postgres>,
    keys: &[BossRewardKey],
    facts: &BossCooldownAdmissionData,
) -> RewardResult<BossCooldownChange> {
    if keys.is_empty()
        || keys.len() > 15
        || !reward_uuid(&facts.admission)
        || facts.cooldown_s > 30 * 24 * 60 * 60
    {
        return Err(BossRewardStateError::InvalidInput);
    }
    let available_at = reward_deadline(facts.now, facts.cooldown_s)?;
    let sorted: std::collections::BTreeSet<_> = keys.iter().cloned().collect();
    if sorted.len() != keys.len() || keys.iter().any(|key| key.boss != keys[0].boss) {
        return Err(BossRewardStateError::InvalidInput);
    }
    let keys: Vec<_> = sorted.into_iter().collect();
    lock_reward_roots(tx, &keys).await?;
    let next = BossCooldownData {
        available_at,
        admission: facts.admission,
        reset_epoch: facts.reset_epoch,
    };
    let mut replayed = 0;
    for key in &keys {
        if let Some(old) = read_boss_cooldown_data(tx, key).await? {
            if facts.reset_epoch < old.reset_epoch {
                return Err(BossRewardStateError::InvalidInput);
            }
            if old.admission == facts.admission {
                if old != next {
                    return Err(BossRewardStateError::ConflictingAdmission);
                }
                replayed += 1;
            } else if facts.now < old.available_at
                && !(facts.boosted && old.reset_epoch < facts.reset_epoch)
            {
                return Ok(BossCooldownChange::OnCooldown);
            }
        }
    }
    if replayed != 0 {
        return if replayed == keys.len() {
            Ok(BossCooldownChange::Replayed)
        } else {
            Err(BossRewardStateError::InvalidStoredState)
        };
    }
    for key in &keys {
        sqlx::query(
            "INSERT INTO game_character_boss_cooldowns(character_id,boss_key,available_at,admission,reset_epoch) \
             VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5::text::numeric(20,0)) \
             ON CONFLICT(character_id,boss_key) DO UPDATE SET available_at=EXCLUDED.available_at, \
             admission=EXCLUDED.admission,reset_epoch=EXCLUDED.reset_epoch",
        )
        .bind(key.character.as_bytes().as_slice()).bind(key.boss.as_str()).bind(next.available_at)
        .bind(next.admission.as_slice()).bind(next.reset_epoch.to_string())
        .execute(&mut **tx).await?;
    }
    Ok(BossCooldownChange::Applied)
}
