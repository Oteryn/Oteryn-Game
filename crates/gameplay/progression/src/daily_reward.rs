//! DAILY_REWARD0 §3.3 arithmetic. Epochs are supplied by the committed World-reset
//! owner; this module never derives an epoch from a clock or a session generation.

pub const STREAK_MAX: u32 = 100_000;
pub const JOKERS_MAX: u8 = 3;
/// Winter Update 2026 (WU26-07, owner decision 2026-11-11): a seven-day cycle.
pub const CYCLE_DAYS: u8 = 7;
/// DAILY30 lanes stored before WU26-07 hold days 1..30 (migration 0547 still allows them);
/// such a lane resumes on the same weekday of the seven-day cycle.
pub const LEGACY_LANE_DAYS: u8 = 30;
/// The streak bonus rises one point per consecutive claim up to this cap.
pub const PROFICIENCY_BONUS_CAP: u8 = 7;

pub use oteryn_gameplay_content::content::daily_reward::DailyRewardTier as DailyRewardSet;

/// Resolve the authored v2 table; there is no second runtime reward table.
pub fn reward_for_day(day: u8, premium: bool) -> Option<DailyRewardSet> {
    oteryn_gameplay_content::content::daily_reward::DailyRewardContent::catalogue()?
        .reward(day, premium)
}

/// Chance, as a whole percentage, to double weapon proficiency XP: one point per streak
/// day, capped at 7% for Free and Premium alike.
pub fn proficiency_bonus_chance(streak: u32) -> u8 {
    u8::try_from(streak).map_or(PROFICIENCY_BONUS_CAP, |streak| {
        streak.min(PROFICIENCY_BONUS_CAP)
    })
}

/// The seven-day cycle day of a stored lane day, mapping a legacy DAILY30 day 8..30 onto
/// the same weekday. Days 1..7 are unchanged.
pub fn cycle_day(lane_day: u8) -> u8 {
    1 + lane_day.saturating_sub(1) % CYCLE_DAYS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyRewardState {
    pub lane_day: u8,
    pub streak: u32,
    pub last_claim_epoch: Option<u64>,
}

impl Default for DailyRewardState {
    fn default() -> Self {
        Self {
            lane_day: 1,
            streak: 0,
            last_claim_epoch: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyRewardAdvance {
    pub next: DailyRewardState,
    pub claimed_lane_day: u8,
    pub jokers_spent: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DailyRewardRuleError {
    InvalidState,
    InvalidEpoch,
    AlreadyClaimed,
    OlderEpoch,
    InvalidResetTime,
}

/// Jokers cover the entire gap or none of it. Missing epochs never move the lane.
pub fn advance_daily_reward(
    state: DailyRewardState,
    epoch: u64,
    jokers: u8,
) -> Result<DailyRewardAdvance, DailyRewardRuleError> {
    if !(1..=LEGACY_LANE_DAYS).contains(&state.lane_day)
        || state.streak > STREAK_MAX
        || jokers > JOKERS_MAX
        || match state.last_claim_epoch {
            None => state.streak != 0 || state.lane_day != 1,
            Some(last) => last == 0 || state.streak == 0,
        }
    {
        return Err(DailyRewardRuleError::InvalidState);
    }
    if epoch == 0 {
        return Err(DailyRewardRuleError::InvalidEpoch);
    }
    let lane_day = cycle_day(state.lane_day);
    let (streak, jokers_spent, claimed_lane_day) = match state.last_claim_epoch {
        None => (1, 0, lane_day),
        Some(last) => {
            let elapsed = epoch
                .checked_sub(last)
                .ok_or(DailyRewardRuleError::OlderEpoch)?;
            let gap = elapsed
                .checked_sub(1)
                .ok_or(DailyRewardRuleError::AlreadyClaimed)?;
            if gap <= u64::from(jokers) {
                let spent = u8::try_from(gap).map_err(|_| DailyRewardRuleError::InvalidState)?;
                (
                    state
                        .streak
                        .checked_add(1)
                        .ok_or(DailyRewardRuleError::InvalidState)?
                        .min(STREAK_MAX),
                    spent,
                    lane_day,
                )
            } else {
                // A gap the jokers cannot cover restarts the streak and the cycle.
                (1, 0, 1)
            }
        }
    };
    Ok(DailyRewardAdvance {
        next: DailyRewardState {
            lane_day: if claimed_lane_day == CYCLE_DAYS {
                1
            } else {
                claimed_lane_day + 1
            },
            streak,
            last_claim_epoch: Some(epoch),
        },
        claimed_lane_day,
        jokers_spent,
    })
}

/// Most recent Berlin 06:00 reset day, expressed as civil days since 1970-01-01.
/// The caller resolves Europe/Berlin using the upstream timezone database first;
/// seconds are local civil seconds, never a fixed UTC offset or a 24-hour window.
/// This labels reset history; it cannot open a claim window or invent an epoch.
pub fn daily_reward_reset_day(
    berlin_civil_day: i64,
    local_seconds: u32,
) -> Result<i64, DailyRewardRuleError> {
    if local_seconds >= 86_400 {
        return Err(DailyRewardRuleError::InvalidResetTime);
    }
    if local_seconds < 21_600 {
        berlin_civil_day
            .checked_sub(1)
            .ok_or(DailyRewardRuleError::InvalidResetTime)
    } else {
        Ok(berlin_civil_day)
    }
}

#[cfg(test)]
#[path = "daily_reward_tests.rs"]
mod tests;

#[path = "daily_reward/xp_boost.rs"]
pub mod xp_boost;
