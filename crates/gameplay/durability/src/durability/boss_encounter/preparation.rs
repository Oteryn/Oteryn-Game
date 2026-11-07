//! Private preparation retained for PostgreSQL qualification.
use super::*;

// BOSS_RAID0 §6.3/§9/§12. These rows are reward-domain state: no CharacterRevision
// comparison or increment. All helpers remain private DATA preparation. The respawn
// preparation independently validates an admitted Channel contributor, but cannot
// prove a boss death or qualified content. Reset, boosted-boss and activity admission
// facts also have no current native producer here. No helper grants reward authority.
use super::cooldown_storage::*;

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
