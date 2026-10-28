use super::*;

#[test]
fn daily_reward_first_consecutive_wrap_and_ceiling() -> Result<(), DailyRewardRuleError> {
    let first = advance_daily_reward(DailyRewardState::default(), 50, 3)?;
    assert_eq!(first.claimed_lane_day, 1);
    assert_eq!(
        first.next,
        DailyRewardState {
            lane_day: 2,
            streak: 1,
            last_claim_epoch: Some(50)
        }
    );
    assert_eq!(first.jokers_spent, 0);
    let next = advance_daily_reward(first.next, 51, 0)?;
    assert_eq!(next.next.streak, 2);
    let wrap = advance_daily_reward(
        DailyRewardState {
            lane_day: 7,
            streak: STREAK_MAX,
            last_claim_epoch: Some(u64::MAX - 1),
        },
        u64::MAX,
        0,
    )?;
    assert_eq!(wrap.next.lane_day, 1);
    assert_eq!(wrap.next.streak, STREAK_MAX);
    Ok(())
}

#[test]
fn daily_reward_gap_coverage_is_all_or_nothing() -> Result<(), DailyRewardRuleError> {
    let state = DailyRewardState {
        lane_day: 4,
        streak: 20,
        last_claim_epoch: Some(1),
    };
    for (epoch, held, streak, spent) in [
        (2, 0, 21, 0),
        (4, 2, 21, 2),
        (5, 3, 21, 3),
        (5, 2, 1, 0),
        (u64::MAX, 3, 1, 0),
    ] {
        let result = advance_daily_reward(state, epoch, held)?;
        assert_eq!((result.next.streak, result.jokers_spent), (streak, spent));
        assert_eq!(result.claimed_lane_day, 4);
        assert_eq!(result.next.lane_day, 5);
    }
    Ok(())
}

#[test]
fn daily_reward_invalid_and_nonmonotonic_windows_refuse() {
    let state = DailyRewardState {
        lane_day: 2,
        streak: 1,
        last_claim_epoch: Some(10),
    };
    assert_eq!(
        advance_daily_reward(state, 10, 0),
        Err(DailyRewardRuleError::AlreadyClaimed)
    );
    assert_eq!(
        advance_daily_reward(state, 9, 0),
        Err(DailyRewardRuleError::OlderEpoch)
    );
    assert_eq!(
        advance_daily_reward(state, 0, 0),
        Err(DailyRewardRuleError::InvalidEpoch)
    );
    for invalid in [
        DailyRewardState {
            lane_day: 0,
            ..state
        },
        DailyRewardState {
            lane_day: 8,
            ..state
        },
        DailyRewardState {
            streak: STREAK_MAX + 1,
            ..state
        },
        DailyRewardState { streak: 0, ..state },
        DailyRewardState {
            last_claim_epoch: Some(0),
            ..state
        },
        DailyRewardState {
            last_claim_epoch: None,
            ..state
        },
    ] {
        assert_eq!(
            advance_daily_reward(invalid, 11, 0),
            Err(DailyRewardRuleError::InvalidState)
        );
    }
    assert_eq!(
        advance_daily_reward(state, 11, 4),
        Err(DailyRewardRuleError::InvalidState)
    );
}

#[test]
fn daily_reward_reset_day_uses_local_ten_am_with_checked_arithmetic() {
    for (second, expected) in [(0, 19999), (35999, 19999), (36000, 20000), (86399, 20000)] {
        assert_eq!(daily_reward_reset_day(20000, second), Ok(expected));
    }
    assert_eq!(daily_reward_reset_day(0, 0), Ok(-1));
    assert_eq!(
        daily_reward_reset_day(i64::MIN, 0),
        Err(DailyRewardRuleError::InvalidResetTime)
    );
    assert_eq!(daily_reward_reset_day(i64::MAX, 36000), Ok(i64::MAX));
    assert_eq!(
        daily_reward_reset_day(0, 86400),
        Err(DailyRewardRuleError::InvalidResetTime)
    );
}

// Crystal 201189f8 data/modules/scripts/daily_reward/daily_reward.lua (Canary 47dfd51f same),
// read as reference data: the reward day wraps after day 7 independently of the streak
// (:262-268); a missed day is covered only when the held jokers cover the whole gap (:318),
// otherwise the streak restarts (:325) and the reward day continues; at most 3 jokers (:308).
// DIFF: Crystal also wipes the held jokers on an uncovered gap; Oteryn spends none.
#[test]
fn crystal_reward_day_streak_and_joker_rules() -> Result<(), DailyRewardRuleError> {
    assert_eq!(JOKERS_MAX, 3);
    let mut state = DailyRewardState::default();
    let mut days = Vec::new();
    for epoch in 1..=8 {
        let advance = advance_daily_reward(state, epoch, 0)?;
        days.push(advance.claimed_lane_day);
        state = advance.next;
    }
    assert_eq!(days, [1, 2, 3, 4, 5, 6, 7, 1]);
    assert_eq!(state.streak, 8);
    for (gap, jokers, streak, spent) in [(1, 1, 9, 1), (3, 3, 9, 3), (2, 1, 1, 0), (4, 3, 1, 0)] {
        let advance = advance_daily_reward(state, 8 + gap + 1, jokers)?;
        assert_eq!(advance.claimed_lane_day, 2, "gap {gap}");
        assert_eq!(
            (advance.next.streak, advance.jokers_spent),
            (streak, spent),
            "gap {gap} jokers {jokers}"
        );
    }
    Ok(())
}
