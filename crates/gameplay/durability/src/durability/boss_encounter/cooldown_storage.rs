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

pub(crate) const ESSENCE_BOSS: &str = "oteryn:creature.essence_of_malice";

impl crate::durability::DurabilityRoot {
    /// Start the Essence lockout for the exact staged roster, or compensate a failed map commit.
    pub async fn essence_lever_cooldown(
        &self,
        authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
        mut roster: Vec<crate::durability::character_progression::CurrentCharacterGameplayFence>,
        reset_epoch: u64,
        compensate: Option<[u8; 16]>,
    ) -> Result<Option<[u8; 16]>, DurabilityError> {
        use crate::durability::{
            character_authority::assert_recovery_fence,
            character_progression::assert_item_only_fence,
            db::{
                begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
            },
        };
        if !(1..=5).contains(&roster.len()) {
            return Ok(None);
        }
        roster.sort_by_key(|fence| fence.character_id);
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run_with_context(&mut (), move |holder, deadline, _| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let mut keys = Vec::with_capacity(roster.len());
                for member in &roster {
                    if assert_item_only_fence(&mut tx, &member.item_fence(), &node).await?.is_err() {
                        return Ok(None);
                    }
                    keys.push(BossRewardKey::new(member.character_id, ESSENCE_BOSS)
                        .map_err(|_| DurabilityError::InvalidStoredState)?);
                }
                let admission = if let Some(admission) = compensate {
                    compensate_boss_cooldowns_data(&mut tx, &keys, admission)
                        .await.map_err(|_| DurabilityError::InvalidStoredState)?;
                    admission
                } else {
                    let (id, now): (sqlx::types::Uuid, i64) = sqlx::query_as(
                        "SELECT game_character_uuid_v7(), floor(extract(epoch FROM statement_timestamp()) * 1000)::bigint",
                    ).fetch_one(&mut *tx).await?;
                    let admission = *id.as_bytes();
                    let change = set_boss_cooldowns_data(&mut tx, &keys, &BossCooldownAdmissionData {
                        admission, now, reset_epoch, cooldown_s: 20 * 60 * 60, boosted: false,
                    }).await.map_err(|_| DurabilityError::InvalidStoredState)?;
                    if change != BossCooldownChange::Applied {
                        return Ok(None);
                    }
                    admission
                };
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Some(admission))
            })
        }).await
    }
}

// An admission's actual ABANDONED/no-arrival predicate must be locked by its owner first.
// Matching only that admission never refunds a participant's later spent cooldown. This
// helper is private and cannot interpret raw IDs as permission to compensate an admission.
pub(super) async fn compensate_boss_cooldowns_data(
    tx: &mut Transaction<'_, Postgres>,
    keys: &[BossRewardKey],
    admission: [u8; 16],
) -> RewardResult<u64> {
    if keys.is_empty() || keys.len() > 15 || !reward_uuid(&admission) {
        return Err(BossRewardStateError::InvalidInput);
    }
    let sorted: std::collections::BTreeSet<_> = keys.iter().cloned().collect();
    if sorted.len() != keys.len() || keys.iter().any(|key| key.boss != keys[0].boss) {
        return Err(BossRewardStateError::InvalidInput);
    }
    let keys: Vec<_> = sorted.into_iter().collect();
    lock_reward_roots(tx, &keys).await?;
    let mut deleted = 0;
    for key in &keys {
        deleted += sqlx::query(
            "DELETE FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid \
             AND boss_key=$2 AND admission=encode($3,'hex')::uuid",
        )
        .bind(key.character.as_bytes().as_slice())
        .bind(key.boss.as_str())
        .bind(admission.as_slice())
        .execute(&mut **tx)
        .await?
        .rows_affected();
    }
    Ok(deleted)
}
