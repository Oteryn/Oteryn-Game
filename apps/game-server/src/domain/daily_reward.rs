//! DAILY_REWARD0 §3.3 stored lane/streak arithmetic. These values are data, never
//! evidence of a current World reset, UTC grant month, shrine reach or Premium.

pub const STREAK_CEILING: u32 = 100_000;
pub const JOKER_CEILING: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RewardState {
    /// The next reward, not the most recently claimed reward.
    pub lane_day: u8,
    pub streak: u32,
    pub last_claim_epoch: Option<u64>,
}

impl Default for RewardState {
    fn default() -> Self {
        Self {
            lane_day: 1,
            streak: 0,
            last_claim_epoch: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateError {
    InvalidState,
    AlreadyClaimed,
    NonMonotonicEpoch,
}

impl RewardState {
    pub fn validate(self) -> Result<Self, StateError> {
        if !(1..=7).contains(&self.lane_day)
            || self.streak > STREAK_CEILING
            || (self.last_claim_epoch.is_none() != (self.streak == 0))
        {
            return Err(StateError::InvalidState);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimArithmetic {
    pub reward_day: u8,
    pub after: RewardState,
    pub jokers_spent: u8,
    pub jokers_remaining: u8,
}

/// Calculate only; this neither claims an epoch nor spends/grants a joker.
/// `epoch` must eventually come from the independently current World reset owner.
/// `jokers_after_grants` is already-granted, locked Account+World storage data;
/// lazy UTC-month grants are deliberately not inferred from epoch numbers/time.
pub fn plan_claim_arithmetic(
    before: RewardState,
    epoch: u64,
    jokers_after_grants: u8,
) -> Result<ClaimArithmetic, StateError> {
    before.validate()?;
    if jokers_after_grants > JOKER_CEILING {
        return Err(StateError::InvalidState);
    }
    let gap = match before.last_claim_epoch {
        None => None,
        Some(last) if epoch == last => return Err(StateError::AlreadyClaimed),
        Some(last) => Some(
            epoch
                .checked_sub(last)
                .ok_or(StateError::NonMonotonicEpoch)?
                - 1,
        ),
    };
    let covered = gap.is_some_and(|gap| gap <= u64::from(jokers_after_grants));
    let spent = if covered { gap.unwrap_or(0) as u8 } else { 0 };
    Ok(ClaimArithmetic {
        reward_day: before.lane_day,
        after: RewardState {
            lane_day: if before.lane_day == 7 {
                1
            } else {
                before.lane_day + 1
            },
            streak: if covered {
                (before.streak + 1).min(STREAK_CEILING)
            } else {
                1
            },
            last_claim_epoch: Some(epoch),
        },
        jokers_spent: spent,
        jokers_remaining: jokers_after_grants - spent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_first_claim_and_lane_wrap_ignore_missed_epochs() -> Result<(), StateError> {
        let first = plan_claim_arithmetic(RewardState::default(), u64::MAX, 3)?;
        assert_eq!(
            (
                first.reward_day,
                first.after.lane_day,
                first.after.streak,
                first.jokers_spent
            ),
            (1, 2, 1, 0)
        );
        let before = RewardState {
            lane_day: 7,
            streak: 42,
            last_claim_epoch: Some(1),
        };
        let next = plan_claim_arithmetic(before, u64::MAX, 3)?;
        assert_eq!(
            (
                next.reward_day,
                next.after.lane_day,
                next.after.streak,
                next.jokers_remaining
            ),
            (7, 1, 1, 3)
        );
        Ok(())
    }

    #[test]
    fn daily_gap_exactly_covered_spends_all_or_nothing() -> Result<(), StateError> {
        let before = RewardState {
            lane_day: 4,
            streak: 99,
            last_claim_epoch: Some(10),
        };
        for jokers in 0..=3 {
            let covered = plan_claim_arithmetic(before, 11 + u64::from(jokers), jokers)?;
            assert_eq!(
                (
                    covered.after.streak,
                    covered.jokers_spent,
                    covered.jokers_remaining
                ),
                (100, jokers, 0)
            );
            let uncovered = plan_claim_arithmetic(before, 12 + u64::from(jokers), jokers)?;
            assert_eq!(
                (
                    uncovered.after.streak,
                    uncovered.jokers_spent,
                    uncovered.jokers_remaining
                ),
                (1, 0, jokers)
            );
        }
        Ok(())
    }

    #[test]
    fn daily_epoch_replay_restart_regression_and_ceiling() -> Result<(), StateError> {
        let before = RewardState {
            lane_day: 2,
            streak: STREAK_CEILING,
            last_claim_epoch: Some(u64::MAX - 1),
        };
        assert_eq!(
            plan_claim_arithmetic(before, u64::MAX - 1, 3),
            Err(StateError::AlreadyClaimed)
        );
        assert_eq!(
            plan_claim_arithmetic(before, 0, 3),
            Err(StateError::NonMonotonicEpoch)
        );
        assert_eq!(
            plan_claim_arithmetic(before, u64::MAX, 0)?.after.streak,
            STREAK_CEILING
        );
        for bad in [
            RewardState {
                lane_day: 0,
                ..before
            },
            RewardState {
                lane_day: 8,
                ..before
            },
            RewardState {
                streak: STREAK_CEILING + 1,
                ..before
            },
            RewardState {
                streak: 0,
                ..before
            },
            RewardState {
                last_claim_epoch: None,
                ..before
            },
        ] {
            assert_eq!(
                plan_claim_arithmetic(bad, u64::MAX, 0),
                Err(StateError::InvalidState)
            );
        }
        assert_eq!(
            plan_claim_arithmetic(before, u64::MAX, 4),
            Err(StateError::InvalidState)
        );
        Ok(())
    }
}
