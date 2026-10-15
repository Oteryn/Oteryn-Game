//! Detached PVP-1b arithmetic for PARTY-PVP-0 §6.2 and §9; not registered or activated.
//!
//! Thresholds are the accepted manual lower bounds, with numeric provenance still
//! `PARITY_PENDING` (`PARTYPVP0-RL-14`/`-15`); this is not Global parity qualification.
//! Durations follow `PARTYPVP0-RL-16`. No later unmerged gap-closure proposal is applied.
//!
//! Inputs are unqualified data, not current authority. A future death-transaction adapter must
//! establish contributor/World identity, classification, complete and deduplicated history,
//! the new occurrence, database time and current fences. No adapter or durable writer exists
//! here, and the result neither grants authority nor authorizes a mutation.
//! Only the caller can establish that these are unjustified Open-PvP points (§9); this helper
//! does not choose a World type, classify kills or enable PvP.

use crate::durability::pvp_state::PvpSkull;

const DAY_MS: u64 = 24 * 60 * 60 * 1_000;
const WINDOWS_MS: [u64; 3] = [DAY_MS, 7 * DAY_MS, 30 * DAY_MS];
const RED_THRESHOLDS_MILLI: [u64; 3] = [3_000, 5_000, 10_000];
const BLACK_THRESHOLDS_MILLI: [u64; 3] = [6_000, 10_000, 20_000];

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
/// Rolling windows use `0 <= age < duration`, i.e. `(now - duration, now]`. §9 does not settle
/// the exact endpoint: this detached technical assumption still needs architect acceptance
/// before integration. Expiry itself follows §6.2's explicit strict `until > now` rule.
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
        PvpSkull::Red => Some(30),
        PvpSkull::Black => Some(45),
    };
    let until_ms = days
        .map(|days| now_ms.checked_add(days * DAY_MS).ok_or(E::DeadlineOverflow))
        .transpose()?;
    Ok(SkullEvaluation {
        state: SkullState { skull, until_ms },
        points_milli,
    })
}

#[cfg(test)]
#[path = "skull_tests.rs"]
mod tests;
