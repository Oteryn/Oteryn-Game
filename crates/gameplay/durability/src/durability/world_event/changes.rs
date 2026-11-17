//! WORLDCHG-Q. One World/reset decision, reused by every Channel and quest lane.
//! Mutation helpers join the caller's fenced semantic transaction; they never commit,
//! debit Items, grant rewards, or authorize a held title's activation.
use super::*;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use oteryn_gameplay_world_change::{
    daily::{DailyCandidates, DailyWorldChanges},
    owner::{CATALOGUE_JSON, ContributionKind, add_contribution},
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};
mod bindings;
pub mod callbacks;
pub mod counters;

/// The day after the reset `reset_epoch` of `world`, on the UTC calendar at `unix_ms`.
pub fn roll(
    world: WorldId,
    reset_epoch: u64,
    unix_ms: i64,
) -> Result<DailyWorldChanges, DurabilityError> {
    let candidates = authored_candidates()?;
    let mut hash = Sha256::new();
    hash.update(b"oteryn.world_change.daily.v1\0");
    hash.update(world.as_bytes());
    let root = GameplayDecisionRoot::from_bytes(hash.finalize().into());
    let mut occurrence = [0; 16];
    occurrence[8..].copy_from_slice(&reset_epoch.to_be_bytes());
    let (month, day) = utc_month_day(unix_ms);
    Ok(DailyWorldChanges::select(
        reset_epoch,
        month,
        day,
        candidates,
        |purpose| {
            // A purpose is never empty, so the draw cannot fail; a failure would read as "no".
            deterministic_decision_u64(
                &root,
                DecisionOccurrenceId::from_bytes(occurrence),
                purpose,
                0,
            )
            .unwrap_or(u64::MAX)
        },
    ))
}

/// Only READY entries with existing physical bindings may become daily candidates.
/// New keys or malformed authored probabilities refuse activation instead of inventing effects.
fn authored_candidates() -> Result<DailyCandidates, DurabilityError> {
    let catalogue: serde_json::Value =
        serde_json::from_str(CATALOGUE_JSON).map_err(|_| DurabilityError::InvalidStoredState)?;
    let entries = catalogue["entries"]
        .as_array()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut probabilities = [None; 3];
    let mut grimvale = false;
    let mut stampede = None;
    let mut fire_earth = None;
    for entry in entries
        .iter()
        .filter(|entry| entry["status"] == "READY" || entry["status"] == "QUALIFIED")
    {
        let index = match entry["key"].as_str() {
            Some("stampede") if stampede.is_none() && entry["trigger"]["kind"] == "at_reset" => {
                stampede = entry["trigger"]["chance_permille"]
                    .as_u64()
                    .filter(|chance| *chance <= 1000)
                    .and_then(|chance| u16::try_from(chance).ok());
                if stampede.is_none() {
                    return Err(DurabilityError::InvalidStoredState);
                }
                continue;
            }
            Some("fire_from_the_earth")
                if fire_earth.is_none() && entry["trigger"]["kind"] == "at_reset" =>
            {
                fire_earth = entry["trigger"]["chance_permille"]
                    .as_u64()
                    .filter(|chance| *chance <= 1000)
                    .and_then(|chance| u16::try_from(chance).ok());
                if fire_earth.is_none() {
                    return Err(DurabilityError::InvalidStoredState);
                }
                continue;
            }
            Some(key)
                if key != "their_masters_voice"
                    && counters::CounterKind::from_key(key).is_some() =>
            {
                continue;
            }
            Some("thawing") => 0,
            Some("their_masters_voice") => 1,
            Some("oriental_trader") => {
                if entry["trigger"]["variants"]
                    != serde_json::json!(["ankrahmun", "carlin", "libertybay"])
                {
                    return Err(DurabilityError::InvalidStoredState);
                }
                2
            }
            Some("grimvale")
                if !grimvale
                    && entry["trigger"]["kind"] == "calendar"
                    && entry["trigger"]["day"] == 13 =>
            {
                grimvale = true;
                continue;
            }
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        if probabilities[index].is_some() || entry["trigger"]["kind"] != "at_reset" {
            return Err(DurabilityError::InvalidStoredState);
        }
        let percent = entry["trigger"]["chance_percent"]
            .as_u64()
            .filter(|percent| *percent <= 100)
            .and_then(|percent| u8::try_from(percent).ok())
            .ok_or(DurabilityError::InvalidStoredState)?;
        probabilities[index] = Some(percent);
    }
    let [
        Some(thawing_percent),
        Some(voice_percent),
        Some(trader_percent),
    ] = probabilities
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    if !grimvale {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(DailyCandidates {
        thawing_percent,
        voice_percent,
        trader_percent,
        stampede_permille: stampede.ok_or(DurabilityError::InvalidStoredState)?,
        // Held content never enters a production daily draw.
        fire_earth_permille: fire_earth.unwrap_or(0),
    })
}

/// Authored W6 preconditions, consumed by the existing raid scheduler before any draw/write.
/// This adds no firing owner and does not replace a raid's own schedule or eligibility.
pub fn raid_requirements() -> Result<std::collections::BTreeMap<String, String>, DurabilityError> {
    oteryn_gameplay_world_change::bindings::raid_requirements()
        .map_err(|_| DurabilityError::InvalidStoredState)
}

/// The UTC (month, day) of `unix_ms` (days-from-civil inverted).
pub fn utc_month_day(unix_ms: i64) -> (u8, u8) {
    let days = unix_ms.div_euclid(86_400_000) + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (
        u8::try_from(month).unwrap_or(1),
        u8::try_from(day).unwrap_or(1),
    )
}

/// Called in reset finalization. Upgrade backfill uses only the immutable epoch's
/// activation timestamp, never the wall clock at Channel boot.
pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<DailyWorldChanges, DurabilityError> {
    let existing: Option<(serde_json::Value, bool)> = sqlx::query_as("SELECT decision,native_binding_required FROM game_world_change_days WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_optional(&mut **tx).await?;
    if let Some((existing, required)) = existing {
        let mut day: DailyWorldChanges =
            serde_json::from_value(existing).map_err(|_| DurabilityError::InvalidStoredState)?;
        if day.reset_epoch != epoch {
            return Err(DurabilityError::InvalidStoredState);
        }
        bindings::verify(tx, world, &day, required).await?;
        counters::project(tx, world, &mut day).await?;
        return Ok(day);
    }
    let timestamp: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM activated_at)*1000)::bigint FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
    let mut selected = roll(world, epoch, timestamp)?;
    counters::at_reset(
        tx,
        world,
        epoch,
        selected.fire_from_the_earth,
        timestamp.max(0) as u64,
    )
    .await?;
    counters::project(tx, world, &mut selected).await?;
    let decision =
        serde_json::to_value(selected).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_world_change_days(world_id,reset_epoch,decision,native_binding_required) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,true) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&decision).execute(&mut **tx).await?;
    let stored: serde_json::Value = sqlx::query_scalar("SELECT decision FROM game_world_change_days WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
    if stored != decision {
        return Err(DurabilityError::InvalidStoredState);
    }
    bindings::capture(tx, world, &selected).await?;
    Ok(selected)
}

/// Reads a current decision in a caller-owned transaction. None is uninitialized,
/// stale epoch, or a retiring World. Holding the snapshot confers no authority.
pub async fn load(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Option<DailyWorldChanges>, DurabilityError> {
    lock_admission_relations(tx).await?;
    let current: Option<String> = sqlx::query_scalar("SELECT a.reset_epoch::text FROM game_world_reset_active a WHERE world_id=encode($1,'hex')::uuid AND NOT EXISTS(SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=a.world_id AND j.phase='RETIRING')")
        .bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    if current.as_deref() != Some(epoch.to_string().as_str()) {
        return Ok(None);
    }
    record(tx, world, epoch).await.map(Some)
}

impl DurabilityRoot {
    pub async fn world_changes(
        &self,
        world: WorldId,
        epoch: u64,
    ) -> Result<Option<DailyWorldChanges>, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let day = load(&mut tx, world, epoch).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(day)
                })
            })
            .await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContributionWrite {
    Applied(u64),
    Replayed(u64),
    Rejected,
}

/// The receipt id must be stable across retries, and unique per World/epoch.
/// The caller validates the live Character/session, title eligibility, vote choice,
/// resource custody and debit in this SAME transaction before commit. Pending
/// outcomes are bookkeeping, never activation or proof of a committed debit.
pub async fn contribute(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    world: WorldId,
    epoch: u64,
    operation_id: [u8; 16],
    kind: ContributionKind,
    amount: u32,
) -> Result<ContributionWrite, DurabilityError> {
    assert_recovery_fence(tx, recovery).await?;
    if amount == 0 || operation_id == [0; 16] || load(tx, world, epoch).await?.is_none() {
        return Ok(ContributionWrite::Rejected);
    }
    // The existing admission relation fence serializes contributors with reset, including receipt
    // lookup and tally mutation. Reset selects a new namespace with zero tallies.
    let receipt: Option<(String, i64, String)> = sqlx::query_as("SELECT resource,amount,resulting_tally::text FROM game_world_change_contributions WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND operation_id=$3")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(operation_id.as_slice()).fetch_optional(&mut **tx).await?;
    if let Some((resource, previous_amount, tally)) = receipt {
        return if resource == kind.key() && previous_amount == i64::from(amount) {
            Ok(ContributionWrite::Replayed(
                tally
                    .parse()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            ))
        } else {
            Ok(ContributionWrite::Rejected)
        };
    }
    let previous = tally(tx, world, epoch, kind).await?;
    let Some(next) = add_contribution(previous, amount) else {
        return Ok(ContributionWrite::Rejected);
    };
    sqlx::query("INSERT INTO game_world_change_tallies(world_id,reset_epoch,resource,tally) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4::text::numeric) ON CONFLICT(world_id,reset_epoch,resource) DO UPDATE SET tally=EXCLUDED.tally")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(kind.key()).bind(next.to_string()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_world_change_contributions(world_id,reset_epoch,operation_id,resource,amount,resulting_tally) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$5,$6::text::numeric)")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(operation_id.as_slice()).bind(kind.key()).bind(i64::from(amount)).bind(next.to_string()).execute(&mut **tx).await?;
    Ok(ContributionWrite::Applied(next))
}

/// Historical tally for a specific epoch; absent resources read as zero.
pub async fn tally(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    kind: ContributionKind,
) -> Result<u64, DurabilityError> {
    let value: Option<String> = sqlx::query_scalar("SELECT tally::text FROM game_world_change_tallies WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND resource=$3")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(kind.key()).fetch_optional(&mut **tx).await?;
    value
        .map(|v| v.parse().map_err(|_| DurabilityError::InvalidStoredState))
        .transpose()
        .map(|v| v.unwrap_or(0))
}
