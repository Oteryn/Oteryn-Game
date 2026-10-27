use super::*;

#[test]
fn global_war_terms_and_end_precedence() {
    assert_eq!(validate_guild_war_terms(10, 7), Ok(()));
    assert_eq!(validate_guild_war_terms(1_000, 180), Ok(()));
    assert_eq!(
        validate_guild_war_terms(9, 7),
        Err(GuildRuleError::InvalidWarTerms)
    );
    assert_eq!(
        validate_guild_war_terms(10, 181),
        Err(GuildRuleError::InvalidWarTerms)
    );
    assert_eq!(
        guild_war_end(10, [10, 0], 50, 49),
        Some(GuildWarEnd::FragLimit)
    );
    assert_eq!(
        guild_war_end(10, [0, 0], 50, 50),
        Some(GuildWarEnd::Duration)
    );
    assert_eq!(guild_war_end(10, [9, 9], 50, 49), None);
}

#[test]
fn guild_names_reuse_character_policy_and_key_without_a_second_normalizer()
-> Result<(), InvalidGuildName> {
    assert!(GuildName::parse(&"a".repeat(29)).is_ok());
    for invalid in ["a".repeat(30), "Élan".into(), "Al\u{200b}Dric".into()] {
        assert_eq!(GuildName::parse(&invalid), Err(InvalidGuildName));
    }
    let spaced = GuildName::parse("Al Dric")?;
    let compact = GuildName::parse("ALDRIC")?;
    assert_eq!(spaced.comparison_key(), compact.comparison_key());
    Ok(())
}

#[test]
fn rank_level_and_count_boundaries_are_enforced() {
    assert_eq!(GuildRank::new(0), Err(GuildRuleError::InvalidRankLevel));
    assert_eq!(GuildRank::new(1), Ok(GuildRank::LEADER));
    assert_eq!(GuildRank::new(2), Ok(GuildRank::VICE));
    assert_eq!(GuildRank::new(20).map(GuildRank::level), Ok(20));
    assert_eq!(GuildRank::new(21), Err(GuildRuleError::InvalidRankLevel));
    assert_eq!(
        validate_rank_count(2),
        Err(GuildRuleError::InvalidRankCount)
    );
    assert_eq!(validate_rank_count(3), Ok(()));
    assert_eq!(validate_rank_count(20), Ok(()));
    assert_eq!(
        validate_rank_count(21),
        Err(GuildRuleError::InvalidRankCount)
    );
}

#[test]
fn rank_changes_require_both_strict_inequalities_and_exclusion_requires_leadership()
-> Result<(), GuildRuleError> {
    let member = GuildRank::new(3)?;
    let lower_member = GuildRank::new(4)?;
    assert!(GuildRank::LEADER.may_set_rank(member, GuildRank::VICE));
    assert!(!GuildRank::VICE.may_set_rank(member, GuildRank::VICE));
    assert!(!GuildRank::VICE.may_set_rank(GuildRank::VICE, member));
    assert!(!member.may_set_rank(GuildRank::VICE, lower_member));
    assert!(member.may_set_rank(lower_member, lower_member));
    assert!(GuildRank::VICE.may_exclude(member));
    assert!(!GuildRank::VICE.may_exclude(GuildRank::VICE));
    assert!(!member.may_exclude(lower_member));
    Ok(())
}

#[test]
fn disbanding_and_terminal_states_refuse_even_a_non_curing_change() {
    for state in [GuildState::Disbanding, GuildState::Disbanded] {
        assert_eq!(
            state.ensure_command_allowed(),
            Err(GuildRuleError::GuildDisbanding)
        );
        assert_eq!(
            state.ensure_vice_count_change_allowed(0, 100, None, 0),
            Err(GuildRuleError::GuildDisbanding)
        );
    }
    assert_eq!(GuildState::Forming.ensure_command_allowed(), Ok(()));
    assert_eq!(GuildState::Active.ensure_command_allowed(), Ok(()));
}

#[test]
fn forming_can_activate_at_its_deadline_but_cannot_activate_later() {
    let deadline = 3 * 24 * 60 * 60 * 1_000;
    for count in [4, 5] {
        assert_eq!(
            GuildState::Forming.ensure_vice_count_change_allowed(count, deadline, None, deadline),
            Ok(())
        );
        assert_eq!(
            GuildState::Forming.ensure_vice_count_change_allowed(
                count,
                deadline,
                None,
                deadline + 1
            ),
            Err(GuildRuleError::GuildDeadlinePassed)
        );
    }
    assert_eq!(
        GuildState::Forming.ensure_vice_count_change_allowed(3, deadline, None, deadline + 1),
        Ok(())
    );
}

#[test]
fn active_vice_grace_survives_reconstruction_without_extending_the_deadline() {
    // Each evaluation reconstructs the inputs from the same hypothetical persisted facts;
    // no helper-local start time can extend the grace after a restart.
    let since = 50;
    let deadline = since + 1_209_600_000; // Accepted 14-day grace, in milliseconds.
    for (now, expected) in [
        (deadline - 1, Ok(())),
        (deadline, Ok(())),
        (deadline + 1, Err(GuildRuleError::GuildDeadlinePassed)),
    ] {
        for _reconstruction in 0..2 {
            assert_eq!(
                GuildState::Active.ensure_vice_count_change_allowed(4, 0, Some(since), now),
                expected
            );
        }
    }
    assert_eq!(
        GuildState::Active.ensure_vice_count_change_allowed(3, 0, Some(since), deadline + 1),
        Ok(())
    );
    assert_eq!(
        GuildState::Active.ensure_vice_count_change_allowed(4, 0, None, i64::MAX),
        Ok(())
    );
}

#[test]
fn vice_deadline_comparison_does_not_overflow_at_timestamp_extremes() {
    assert_eq!(
        GuildState::Active.ensure_vice_count_change_allowed(4, 0, Some(i64::MIN), i64::MAX),
        Err(GuildRuleError::GuildDeadlinePassed)
    );
    assert_eq!(
        GuildState::Active.ensure_vice_count_change_allowed(4, 0, Some(i64::MAX), i64::MIN),
        Ok(())
    );
}
