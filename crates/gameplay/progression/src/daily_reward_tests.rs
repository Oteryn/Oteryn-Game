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
            lane_day: 30,
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
        let expected_day = if streak == 1 && spent == 0 { 1 } else { 4 };
        assert_eq!(result.claimed_lane_day, expected_day);
        assert_eq!(result.next.lane_day, expected_day + 1);
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
            lane_day: 31,
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
// read as reference data for the retained all-or-nothing joker behaviour.
// DIFF: Oteryn has a 30-day lane and milestone fallback, and does not wipe held jokers.
#[test]
fn crystal_reward_day_streak_and_joker_rules() -> Result<(), DailyRewardRuleError> {
    assert_eq!(JOKERS_MAX, 3);
    let mut state = DailyRewardState::default();
    let mut days = Vec::new();
    for epoch in 1..=31 {
        let advance = advance_daily_reward(state, epoch, 0)?;
        days.push(advance.claimed_lane_day);
        state = advance.next;
    }
    assert_eq!(days, (1..=30).chain([1]).collect::<Vec<_>>());
    assert_eq!(state.streak, 31);
    for (gap, jokers, streak, spent) in [(1, 1, 32, 1), (3, 3, 32, 3), (2, 1, 1, 0), (4, 3, 1, 0)] {
        let advance = advance_daily_reward(state, 31 + gap + 1, jokers)?;
        assert_eq!(
            advance.claimed_lane_day,
            if spent == 0 && gap > u64::from(jokers) {
                1
            } else {
                2
            },
            "gap {gap}"
        );
        assert_eq!(
            (advance.next.streak, advance.jokers_spent),
            (streak, spent),
            "gap {gap} jokers {jokers}"
        );
    }
    Ok(())
}

#[test]
fn all_thirty_days_have_exact_free_and_premium_rewards() -> Result<(), DailyRewardRuleError> {
    let free_cosmetic = [
        2, 0, 0, 0, 3, 0, 0, 2, 0, 0, 0, 3, 0, 2, 2, 0, 0, 0, 3, 0, 3, 2, 0, 0, 0, 3, 0, 4, 2, 5,
    ];
    let premium_cosmetic = [
        4, 0, 0, 0, 6, 0, 0, 4, 0, 0, 0, 6, 0, 4, 4, 0, 0, 0, 6, 0, 6, 4, 0, 0, 0, 6, 0, 8, 4, 10,
    ];
    for day in 1..=CYCLE_DAYS {
        for (premium, cosmetics) in [(false, free_cosmetic), (true, premium_cosmetic)] {
            let reward = reward_for_day(day, premium).ok_or(DailyRewardRuleError::InvalidState)?;
            assert_eq!(
                reward.cosmetic_tokens,
                cosmetics[usize::from(day - 1)],
                "day {day}, premium {premium}"
            );
            assert_eq!(reward.bounty_reroll_tokens, 1);
            assert_eq!(
                reward.temple_teleport_scrolls,
                u8::from(matches!(day, 3 | 10 | 17 | 24 | 30))
            );
            let expected_wildcards = match day {
                2 | 6 | 9 | 13 | 16 | 20 | 23 | 27 => {
                    if premium {
                        2
                    } else {
                        1
                    }
                }
                21 => 1,
                28 => {
                    if premium {
                        2
                    } else {
                        1
                    }
                }
                _ => 0,
            };
            assert_eq!(
                reward.prey_wildcards, expected_wildcards,
                "day {day}, premium {premium}"
            );
            let (percent, minutes) = match day {
                4 | 11 | 18 | 25 => (if premium { 25 } else { 10 }, 30),
                7 | 14 => (if premium { 50 } else { 20 }, 30),
                21 => (if premium { 50 } else { 20 }, 45),
                28 | 30 => (if premium { 50 } else { 20 }, 60),
                _ => (0, 0),
            };
            assert_eq!(
                (reward.xp_boost_percent, reward.xp_boost_minutes),
                (percent, minutes)
            );
        }
    }
    assert_eq!(reward_for_day(0, false), None);
    assert_eq!(reward_for_day(31, true), None);
    Ok(())
}

#[test]
fn missed_days_fall_back_to_current_cycle_milestones() -> Result<(), DailyRewardRuleError> {
    for (next_day, claimed, streak) in [
        (2, 1, 1),
        (8, 8, 8),
        (15, 15, 15),
        (22, 22, 22),
        (29, 29, 29),
    ] {
        let advance = advance_daily_reward(
            DailyRewardState {
                lane_day: next_day,
                streak: 50,
                last_claim_epoch: Some(10),
            },
            12,
            0,
        )?;
        assert_eq!(
            (
                advance.claimed_lane_day,
                advance.next.lane_day,
                advance.next.streak
            ),
            (claimed, claimed + 1, streak)
        );
    }
    Ok(())
}

#[test]
fn proficiency_chance_reaches_milestones_and_stays_at_ten_percent() {
    for (streak, expected) in [
        (0, 0),
        (1, 1),
        (7, 7),
        (8, 7),
        (14, 8),
        (21, 9),
        (28, 10),
        (30, 10),
        (100_000, 10),
    ] {
        assert_eq!(proficiency_bonus_chance(streak), expected);
    }
}

#[test]
fn fallback_to_seven_then_claims_reach_proficiency_milestones() -> Result<(), DailyRewardRuleError>
{
    let mut state = DailyRewardState {
        lane_day: 13,
        streak: 12,
        last_claim_epoch: Some(20),
    };
    for (epoch, day) in (22..).zip(8..=21) {
        let advance = advance_daily_reward(state, epoch, 0)?;
        assert_eq!(advance.claimed_lane_day, day);
        assert_eq!(advance.next.streak, u32::from(day));
        assert_eq!(advance.next.lane_day, day + 1);
        if day == 14 {
            assert_eq!(proficiency_bonus_chance(advance.next.streak), 8);
        }
        if day == 21 {
            assert_eq!(proficiency_bonus_chance(advance.next.streak), 9);
        }
        state = advance.next;
    }
    Ok(())
}

#[test]
fn authored_catalogue_matches_all_owner_rules() -> Result<(), Box<dyn std::error::Error>> {
    let content: oteryn_gameplay_content::content::daily_reward::DailyRewardContent =
        serde_json::from_str(include_str!(
            "../../../../content/interactions/daily-rewards.json"
        ))?;
    assert_eq!(content.days.len(), 30);
    for day in &content.days {
        for premium in [false, true] {
            assert_eq!(
                reward_for_day(day.day, premium),
                Some(if premium { day.premium } else { day.free })
            );
        }
    }
    // The exhaustive independent owner-rule assertions above check every reward field
    // for all 60 variants, so changing JSON alone cannot change accepted rules.
    all_thirty_days_have_exact_free_and_premium_rewards().map_err(|e| format!("{e:?}"))?;
    Ok(())
}
