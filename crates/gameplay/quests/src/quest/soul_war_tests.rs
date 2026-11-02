use super::*;

fn started() -> SoulWarState {
    SoulWarState {
        teleport_access: true,
        ..SoulWarState::default()
    }
}

fn all_bosses() -> SoulWarState {
    SoulWarState {
        teleport_access: true,
        defeated_bosses: ALL_MINI_BOSSES,
        first_taint_at: Some(1_000),
        ..SoulWarState::default()
    }
}

#[test]
fn soul_war_acceptance_is_separate_from_the_level_250_entry_gate() {
    assert_eq!(
        SoulWarState::default().apply(SoulWarTransition::AcceptTask, 10),
        Ok(started())
    );
    assert_eq!(MINIMUM_LEVEL, 250);
}

#[test]
fn soul_war_five_distinct_bosses_work_in_every_order_without_extending_the_clock() {
    fn walk(state: SoulWarState, remaining: &[SoulWarBoss], count: u8) {
        if remaining.is_empty() {
            assert!(state.chain_ready());
            assert_eq!(state.taint_count(2_000), 5);
        }
        for (index, boss) in remaining.iter().copied().enumerate() {
            let result = state.apply(
                SoulWarTransition::BossDefeated(boss),
                1_000 + i64::from(count),
            );
            let Ok(next) = result else {
                assert_eq!(result, Ok(state));
                return;
            };
            assert_eq!(next.first_taint_at, Some(1_000));
            assert_eq!(next.taint_count(2_000), count + 1);
            assert_eq!(
                next.apply(SoulWarTransition::BossDefeated(boss), 2_000),
                Ok(next)
            );
            let mut rest = remaining.to_vec();
            rest.remove(index);
            walk(next, &rest, count + 1);
        }
    }
    walk(
        started(),
        &[
            SoulWarBoss::Malice,
            SoulWarBoss::Hatred,
            SoulWarBoss::Spite,
            SoulWarBoss::Cruelty,
            SoulWarBoss::Greed,
        ],
        0,
    );
}

#[test]
fn soul_war_access_is_required_and_final_flags_are_checked_at_entry() {
    for boss in [SoulWarBoss::Malice, SoulWarBoss::Megalomania] {
        assert_eq!(
            SoulWarState::default().apply(SoulWarTransition::BossDefeated(boss), 10),
            Err(SoulWarRefusal::AccessRequired)
        );
    }
    let completed = SoulWarState {
        megalomania_completed: true,
        ..started()
    };
    for mask in 0..ALL_MINI_BOSSES {
        let state = SoulWarState {
            defeated_bosses: mask,
            first_taint_at: (mask != 0).then_some(1_000),
            ..started()
        };
        assert!(
            !state.chain_ready(),
            "incomplete flags cannot authorize entry"
        );
        // A source-qualified final death is a different event from entry. Only
        // the native receipt consumer can issue this transition in production.
        assert_eq!(
            state.apply(
                SoulWarTransition::BossDefeated(SoulWarBoss::Megalomania),
                2_000
            ),
            Ok(completed)
        );
    }
    let expired = all_bosses().expired(1_000 + TAINT_DURATION_SECONDS);
    assert!(!expired.chain_ready());
    assert_eq!(
        all_bosses().apply(
            SoulWarTransition::BossDefeated(SoulWarBoss::Megalomania),
            1_000 + TAINT_DURATION_SECONDS
        ),
        Ok(completed)
    );
}

#[test]
fn soul_war_expiry_is_exactly_fourteen_days_from_first_distinct_kill() {
    let state = all_bosses();
    assert_eq!(state.taint_count(1_000 + TAINT_DURATION_SECONDS - 1), 5);
    assert_eq!(state.taint_count(1_000 + TAINT_DURATION_SECONDS), 0);
    assert_eq!(
        state.apply(SoulWarTransition::Refresh, 1_000 + TAINT_DURATION_SECONDS),
        Ok(started())
    );
    let now = 1_000 + TAINT_DURATION_SECONDS;
    assert_eq!(
        state.apply(SoulWarTransition::BossDefeated(SoulWarBoss::Greed), now),
        Ok(SoulWarState {
            defeated_bosses: SoulWarBoss::Greed.bit(),
            first_taint_at: Some(now),
            ..started()
        })
    );
}

#[test]
fn soul_war_cleanse_preserves_permanent_completion_and_reward_history() {
    let state = SoulWarState {
        megalomania_completed: true,
        final_reward_claimed: true,
        ..all_bosses()
    };
    assert_eq!(
        state.apply(SoulWarTransition::Cleanse, 2_000),
        Ok(SoulWarState {
            megalomania_completed: true,
            final_reward_claimed: true,
            ..started()
        })
    );
}

#[test]
fn soul_war_final_success_resets_cycle_and_exposes_only_one_reward() {
    let completed = SoulWarState {
        megalomania_completed: true,
        ..started()
    };
    assert_eq!(
        all_bosses().apply(
            SoulWarTransition::BossDefeated(SoulWarBoss::Megalomania),
            2_000
        ),
        Ok(completed)
    );
    let claimed = SoulWarState {
        final_reward_claimed: true,
        ..completed
    };
    assert_eq!(
        completed.apply(SoulWarTransition::ClaimReward, 2_000),
        Ok(claimed)
    );
    assert_eq!(
        claimed.apply(SoulWarTransition::ClaimReward, 2_000),
        Err(SoulWarRefusal::RewardAlreadyClaimed)
    );
    assert_eq!(
        started().apply(SoulWarTransition::ClaimReward, 2_000),
        Err(SoulWarRefusal::CompletionRequired)
    );
    assert_eq!(
        started().apply(SoulWarTransition::ReportCompletion, 2_000),
        Err(SoulWarRefusal::CompletionRequired)
    );
    assert_eq!(
        completed.apply(SoulWarTransition::ReportCompletion, 2_000),
        Ok(completed)
    );
}

#[test]
fn soul_war_failed_final_uses_exactly_two_distinct_flags_and_retains_expiry() {
    assert_eq!(
        started().apply(
            SoulWarTransition::FailedMegalomania { removed_bosses: 3 },
            2_000
        ),
        Err(SoulWarRefusal::BossChainRequired)
    );
    let mut seen = std::collections::BTreeSet::new();
    for draw in 0..10 {
        let pair = failure_pair(draw);
        assert!(seen.insert(pair));
        let next = all_bosses().apply(
            SoulWarTransition::FailedMegalomania {
                removed_bosses: pair,
            },
            2_000,
        );
        assert_eq!(
            next,
            Ok(SoulWarState {
                defeated_bosses: ALL_MINI_BOSSES & !pair,
                ..all_bosses()
            })
        );
        if let Ok(next) = next {
            assert_eq!(next.taint_count(2_000), 3);
            assert!(!next.chain_ready());
        }
        assert_eq!(failure_pair(draw + 10), pair);
    }
    for removed_bosses in [0, 1, 7, 32, 255] {
        assert_eq!(
            all_bosses().apply(
                SoulWarTransition::FailedMegalomania { removed_bosses },
                2_000
            ),
            Err(SoulWarRefusal::InvalidFailurePair)
        );
    }
}

#[test]
fn soul_war_xp_uses_exact_powers_and_one_floor_not_the_rounded_donor_map() {
    assert_eq!(
        (0..=5)
            .map(|n| taint_experience(1_000_000, n))
            .collect::<Vec<_>>(),
        vec![
            Ok(1_000_000),
            Ok(1_045_000),
            Ok(1_092_025),
            Ok(1_141_166),
            Ok(1_192_518),
            Ok(1_246_181)
        ]
    );
    assert_eq!(taint_experience(1, 5), Ok(1));
    assert_eq!(taint_experience(0, 5), Ok(0));
    assert_eq!(
        taint_experience(i64::MAX, 5),
        Err(SoulWarRefusal::InvalidState)
    );
    assert_eq!(taint_experience(10, 6), Err(SoulWarRefusal::InvalidState));
    assert_eq!(taint_experience(-1, 0), Err(SoulWarRefusal::InvalidState));
}

#[test]
fn soul_war_bonus_has_a_closed_source_species_list() {
    assert!(qualifies_for_taint_experience(
        "oteryn:creature.bony_sea_devil"
    ));
    assert!(qualifies_for_taint_experience(
        "oteryn:creature.monk_s_apparition"
    ));
    assert!(qualifies_for_taint_experience(
        "oteryn:creature.goshnar_s_megalomania_green"
    ));
    for key in [
        "oteryn:creature.rat",
        "oteryn:creature.bakragore",
        "bony_sea_devil",
        "oteryn:creature.goshnar_s_megalomania_purple",
        "",
    ] {
        assert!(!qualifies_for_taint_experience(key));
    }
    assert_eq!(
        SoulWarBoss::from_creature("oteryn:creature.goshnar_s_megalomania_purple"),
        None
    );
}

#[test]
fn soul_war_invalid_states_and_clock_are_refused_without_mutation() {
    for state in [
        SoulWarState {
            defeated_bosses: 1,
            ..started()
        },
        SoulWarState {
            first_taint_at: Some(0),
            ..started()
        },
        SoulWarState {
            defeated_bosses: 32,
            first_taint_at: Some(1),
            ..started()
        },
        SoulWarState {
            final_reward_claimed: true,
            ..started()
        },
    ] {
        assert_eq!(
            state.apply(SoulWarTransition::Refresh, 20),
            Err(SoulWarRefusal::InvalidState)
        );
    }
    assert_eq!(
        started().apply(SoulWarTransition::Refresh, -1),
        Err(SoulWarRefusal::InvalidTime)
    );
}
