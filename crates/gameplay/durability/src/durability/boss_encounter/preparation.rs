//! Private preparation retained for PostgreSQL qualification.
use super::*;

// BOSS_RAID0 §6.3/§9/§12. These rows are reward-domain state: no CharacterRevision
// comparison or increment. All helpers remain private DATA preparation. The respawn
// preparation independently validates an admitted Channel contributor, but cannot
// prove a boss death or qualified content. Reset, boosted-boss and activity admission
// facts also have no current native producer here. No helper grants reward authority.
type RewardResult<T> = std::result::Result<T, BossRewardStateError>;

#[derive(Debug)]
enum BossRewardStateError {
    InvalidInput,
    InvalidStoredState,
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
            Self::AuthorityRejected => f.write_str("boss contributor authority rejected"),
            Self::ConflictingAdmission => f.write_str("boss admission reused with different state"),
            Self::Unavailable(error) => write!(f, "boss reward state unavailable: {error:?}"),
        }
    }
}
impl std::error::Error for BossRewardStateError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct BossRewardKey {
    character: CharacterId,
    boss: ProductionKey,
}
impl BossRewardKey {
    fn new(character: CharacterId, boss: &str) -> RewardResult<Self> {
        Ok(Self {
            character,
            boss: ProductionKey::new(boss).map_err(|_| BossRewardStateError::InvalidInput)?,
        })
    }
}

/// Pending transaction result only; it is neither a committed death nor reward authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BossEligibilityClaim {
    Claimed,
    Ineligible,
}

/// Private respawn-eligibility storage preparation with a genuine contributor fence.
/// No current native death/content owner calls this. The future owner must bind it to
/// a qualified boss death before making any production entry reachable.
/// It must retain its recovery seal, hold the existing death occurrence first,
/// check its immutable death receipt for replay before calling, and compose the claim,
/// reward draw and Bosstiary receipt in this same semantic transaction/deadline/commit.
/// A rollback releases the claim; an ambiguous commit must reconcile that death receipt,
/// never retry this as a new death. No standalone transaction/commit is offered here.
///
/// Content supplies the qualified boss key and spawn's actual min_s, as for the clock's
/// death planner. This entry supplies neither content admission nor death legitimacy.
/// Terminal/post-grace and Instance scopes fail closed under the native session fence;
/// §7 forbids crediting post-grace ahead of an accepted writer for the actor's own death.
async fn claim_respawn_eligibility_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &crate::durability::item_transfer::CurrentCharacterItemFence,
    boss: &str,
    min_s: u64,
) -> RewardResult<BossEligibilityClaim> {
    let key = BossRewardKey::new(fence.character_id, boss)?;
    reward_deadline(0, min_s)?;
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    let history_valid: bool =
        sqlx::query_scalar("SELECT game_runtime_scope_assignment_history_valid()")
            .fetch_one(&mut **tx)
            .await?;
    if !history_valid {
        return Err(BossRewardStateError::InvalidStoredState);
    }
    crate::durability::character_progression::assert_item_only_fence(tx, fence, node)
        .await?
        .map_err(|_| BossRewardStateError::AuthorityRejected)?;
    // All blocking authority/Character-root locks precede the DB time sample. No
    // caller-supplied timestamp or process-local timer can open an eligibility window.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    claim_open_world_eligibility_data(tx, &key, now, OpenWorldEligibilityData::Respawn { min_s })
        .await
}

fn reward_deadline(now: i64, seconds: u64) -> RewardResult<i64> {
    if now < 0 {
        return Err(BossRewardStateError::InvalidInput);
    }
    i64::try_from(seconds)
        .ok()
        .and_then(|seconds| seconds.checked_mul(1000))
        .and_then(|delta| now.checked_add(delta))
        .ok_or(BossRewardStateError::InvalidInput)
}

fn reward_uuid(value: &[u8; 16]) -> bool {
    value[6] >> 4 == 7 && value[8] & 0xc0 == 0x80
}

// Character roots serialize missing singleton/cooldown rows too. Sorting distinct roots
// before rows preserves §12 even for a group. This is only a lock, never an authority check.
async fn lock_reward_roots(
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

#[derive(Debug, Clone, Copy)]
enum OpenWorldEligibilityData {
    Respawn { min_s: u64 },
    AtReset { epoch: u64 },
}

// Private storage mechanics. Raw epoch/time values do not prove a current reset/death.
// The future at-reset caller must independently resolve the current WorldReset and use
// the same §7 fence as its own death writer; no exported at-reset activation exists.
async fn claim_open_world_eligibility_data(
    tx: &mut Transaction<'_, Postgres>,
    key: &BossRewardKey,
    now: i64,
    policy: OpenWorldEligibilityData,
) -> RewardResult<BossEligibilityClaim> {
    let next = match policy {
        OpenWorldEligibilityData::Respawn { min_s } => reward_deadline(now, min_s)?,
        OpenWorldEligibilityData::AtReset { .. } if now >= 0 => 0,
        _ => return Err(BossRewardStateError::InvalidInput),
    };
    lock_reward_roots(tx, std::slice::from_ref(key)).await?;
    sqlx::query(
        "INSERT INTO game_character_boss_open_world_eligibility \
         (character_id,boss_key,next_eligible_at,last_credited_epoch) \
         VALUES(encode($1,'hex')::uuid,$2,0,NULL) ON CONFLICT DO NOTHING",
    )
    .bind(key.character.as_bytes().as_slice())
    .bind(key.boss.as_str())
    .execute(&mut **tx)
    .await?;
    let row = sqlx::query(
        "SELECT next_eligible_at,last_credited_epoch::text \
         FROM game_character_boss_open_world_eligibility \
         WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 FOR UPDATE",
    )
    .bind(key.character.as_bytes().as_slice())
    .bind(key.boss.as_str())
    .fetch_one(&mut **tx)
    .await?;
    let before: i64 = row.try_get("next_eligible_at")?;
    let last_epoch: Option<String> = row.try_get("last_credited_epoch")?;
    let last_epoch = last_epoch
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| BossRewardStateError::InvalidStoredState)
        })
        .transpose()?;
    if before < 0 {
        return Err(BossRewardStateError::InvalidStoredState);
    }
    let (next, epoch) = match policy {
        OpenWorldEligibilityData::Respawn { .. } if now >= before => (next, last_epoch),
        OpenWorldEligibilityData::AtReset { epoch }
            if last_epoch.is_none_or(|last| last < epoch) =>
        {
            (before, Some(epoch))
        }
        _ => return Ok(BossEligibilityClaim::Ineligible),
    };
    sqlx::query(
        "UPDATE game_character_boss_open_world_eligibility \
         SET next_eligible_at=$3,last_credited_epoch=$4::text::numeric(20,0) \
         WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2",
    )
    .bind(key.character.as_bytes().as_slice())
    .bind(key.boss.as_str())
    .bind(next)
    .bind(epoch.map(|value| value.to_string()))
    .execute(&mut **tx)
    .await?;
    Ok(BossEligibilityClaim::Claimed)
}

// Existing firing_id/admission identity only; no new eligibility-window ID is derived.
// Caller must check the death receipt before this, and compose the draw/receipt atomically.
async fn claim_reference_eligibility_data(
    tx: &mut Transaction<'_, Postgres>,
    key: &BossRewardKey,
    eligibility_ref: [u8; 16],
) -> RewardResult<BossEligibilityClaim> {
    if !reward_uuid(&eligibility_ref) {
        return Err(BossRewardStateError::InvalidInput);
    }
    lock_reward_roots(tx, std::slice::from_ref(key)).await?;
    let inserted = sqlx::query(
        "INSERT INTO game_character_boss_eligibility(character_id,boss_key,eligibility_ref) \
         VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid) ON CONFLICT DO NOTHING",
    )
    .bind(key.character.as_bytes().as_slice())
    .bind(key.boss.as_str())
    .bind(eligibility_ref.as_slice())
    .execute(&mut **tx)
    .await?
    .rows_affected();
    Ok(if inserted == 1 {
        BossEligibilityClaim::Claimed
    } else {
        BossEligibilityClaim::Ineligible
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BossCooldownData {
    available_at: i64,
    admission: [u8; 16],
    reset_epoch: u64,
}

async fn read_boss_cooldown_data(
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
struct BossCooldownAdmissionData {
    admission: [u8; 16],
    now: i64,
    reset_epoch: u64,
    cooldown_s: u64,
    boosted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BossCooldownChange {
    Applied,
    Replayed,
    OnCooldown,
}

// No partial group write on a refusal: validate and lock every root/row in deterministic
// order, then preflight the entire group before the first write. Allocation ADMITTING and
// cooldowns still must commit together in the real lever owner, which this does not replace.
async fn set_boss_cooldowns_data(
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

// An admission's actual ABANDONED/no-arrival predicate must be locked by its owner first.
// Matching only that admission never refunds a participant's later spent cooldown. This
// helper is private and cannot interpret raw IDs as permission to compensate an admission.
async fn compensate_boss_cooldowns_data(
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

#[cfg(test)]
#[allow(clippy::expect_used)]
#[path = "tests.rs"]
mod tests;
