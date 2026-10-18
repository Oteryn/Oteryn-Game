//! PVP-1b skull arithmetic for PARTY-PVP-0 §6.2 and §9, evaluated inside the victim's death
//! transaction ([`crate::durability::pvp_state`]); the policy rows are
//! `rulesets/pvp/skulls/skull-policy.json`.
//!
//! Mounted from `durability/pvp_state.rs`, because the death transaction is compiled into
//! targets that hold the durability tree without the combat tree.
//!
//! Thresholds are the manual's lower bounds (`PARTYPVP0-RL-14`/`-15`); their exact values are
//! not given by the manual, so they are the Canary values and stay `OTS_HYPOTHESIS_ONLY` in the
//! ruleset. Durations follow `PARTYPVP0-RL-16` (manual). This is not Global parity
//! qualification.
//!
//! Inputs are the caller's data: the death transaction establishes contributor/World identity,
//! the runtime classification, complete history, the new occurrence, database time and the
//! fences. Only the caller can establish that these are unjustified Open-PvP points (§9); this
//! helper does not choose a World type, classify kills or enable PvP.

use crate::durability::pvp_state::PvpSkull;

const DAY_MS: u64 = 24 * 60 * 60 * 1_000;
const WINDOWS_MS: [u64; 3] = [DAY_MS, 7 * DAY_MS, 30 * DAY_MS];
/// The longest window: older points never count (they stay until `PARTYPVP0-RL-23`).
pub(crate) const HISTORY_WINDOW_MS: u64 = WINDOWS_MS[2];
const RED_THRESHOLDS_MILLI: [u64; 3] = [3_000, 5_000, 10_000];
const BLACK_THRESHOLDS_MILLI: [u64; 3] = [6_000, 10_000, 20_000];
const RED_DAYS: u64 = 30;
const BLACK_DAYS: u64 = 45;
/// `PARTYPVP0-RL-13`: a kill sets the database clock + 15 minutes for every contributor.
pub(crate) const KILL_BLOCK_MS: u64 = 15 * 60 * 1_000;
/// §8.3: at most 16 full and 64 compact ledger rows, so at most 80 contributors.
pub(crate) const MAX_DEATH_CONTRIBUTORS: usize = 80;
/// `PARTYPVP0-RL-18`: a kill counts in full for at most 5 damage contributors.
const FULL_CREDIT_DAMAGE_CONTRIBUTORS: u64 = 5;
const DAMAGE_POINTS_MILLI: u64 = 1_000;
/// `PARTYPVP0-RL-19` (`PARITY_PENDING`): an assist-only contributor gains half.
const ASSIST_POINTS_MILLI: u16 = 500;

/// §9 points of one unjustified contributor: 1,000 milli for a damage contributor, 500 for an
/// assist-only one; with more than 5 damage contributors each gets `1,000 × 5 / n`, rounded
/// down. `None` when `damage_contributors` is out of range (0 for a damage contributor, more
/// than [`MAX_DEATH_CONTRIBUTORS`]).
pub(crate) fn unjustified_points_milli(damage: bool, damage_contributors: usize) -> Option<u16> {
    let n = u64::try_from(damage_contributors).ok()?;
    if n > MAX_DEATH_CONTRIBUTORS as u64 || (damage && n == 0) {
        return None;
    }
    if !damage {
        return Some(ASSIST_POINTS_MILLI);
    }
    let points = if n > FULL_CREDIT_DAMAGE_CONTRIBUTORS {
        DAMAGE_POINTS_MILLI * FULL_CREDIT_DAMAGE_CONTRIBUTORS / n
    } else {
        DAMAGE_POINTS_MILLI
    };
    u16::try_from(points).ok()
}

/// A point row supplied by the caller, excluding the new point increment.
/// Timestamps share the caller's millisecond time domain; they are not clock/commit evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkullPoint {
    pub(crate) committed_at_ms: u64,
    pub(crate) points_milli: u16,
}

/// Unqualified stored values, not a live/locked persistence row or an authority token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkullState {
    pub(crate) skull: PvpSkull,
    pub(crate) until_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkullEvaluation {
    pub(crate) state: SkullState,
    /// Rolling 24-hour, 7-day and 30-day totals, including the new increment exactly once.
    pub(crate) points_milli: [u64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkullEvaluationError {
    InvalidState,
    InvalidPoints,
    FuturePoint,
    PointsOverflow,
    DeadlineOverflow,
}

/// Evaluate one contributor that gained `new_points_milli` (1..=1,000) at `now_ms`.
/// `history` excludes that increment; it is added to all three windows exactly once here.
/// This is an arithmetic input convention, not evidence that a new death/commit occurred.
/// No-points calls are refused rather than refreshing a deadline or evaluating old points.
///
/// Rolling windows use `0 <= age < duration`, i.e. `(now - duration, now]`: each point counts
/// from its committed timestamp and the window end is exclusive (control-plane decision D1443).
/// Expiry itself follows §6.2's explicit strict `until > now` rule.
pub(crate) fn evaluate_skull(
    now_ms: u64,
    current: SkullState,
    history: &[SkullPoint],
    new_points_milli: u16,
) -> Result<SkullEvaluation, SkullEvaluationError> {
    use SkullEvaluationError as E;
    match (current.skull, current.until_ms) {
        (PvpSkull::None, None) | (PvpSkull::Red | PvpSkull::Black, Some(_)) => {}
        _ => return Err(E::InvalidState),
    }
    if !(1..=1_000).contains(&new_points_milli) {
        return Err(E::InvalidPoints);
    }
    let mut points_milli = [u64::from(new_points_milli); 3];
    for row in history {
        if !(1..=1_000).contains(&row.points_milli) {
            return Err(E::InvalidPoints);
        }
        let age = now_ms
            .checked_sub(row.committed_at_ms)
            .ok_or(E::FuturePoint)?;
        for (total, duration) in points_milli.iter_mut().zip(WINDOWS_MS) {
            if age < duration {
                *total = total
                    .checked_add(u64::from(row.points_milli))
                    .ok_or(E::PointsOverflow)?;
            }
        }
    }
    let reached = |thresholds: [u64; 3]| {
        points_milli
            .iter()
            .zip(thresholds)
            .any(|(&total, threshold)| total >= threshold)
    };
    let active = if current.until_ms.is_some_and(|until| until > now_ms) {
        current.skull
    } else {
        PvpSkull::None
    };
    let skull = if active == PvpSkull::Black || reached(BLACK_THRESHOLDS_MILLI) {
        PvpSkull::Black
    } else if active == PvpSkull::Red || reached(RED_THRESHOLDS_MILLI) {
        PvpSkull::Red
    } else {
        PvpSkull::None
    };
    let days = match skull {
        PvpSkull::None => None,
        PvpSkull::Red => Some(RED_DAYS),
        PvpSkull::Black => Some(BLACK_DAYS),
    };
    let until_ms = days
        .map(|days| now_ms.checked_add(days * DAY_MS).ok_or(E::DeadlineOverflow))
        .transpose()?;
    // The installed state guard also forbids shortening an unexpired skull, including
    // during an upgrade. Check the reset arithmetic before preserving a later deadline.
    let until_ms = until_ms.map(|reset| {
        current
            .until_ms
            .filter(|&until| until > now_ms)
            .map_or(reset, |until| reset.max(until))
    });
    Ok(SkullEvaluation {
        state: SkullState { skull, until_ms },
        points_milli,
    })
}

#[cfg(test)]
#[path = "skull_tests.rs"]
mod tests;
