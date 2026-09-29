#![allow(clippy::expect_used)]
//! Structural tests use a synthetic NON_REFERENCE policy. Target threshold parity is UNKNOWN.

use oteryn_game_server::domain::progression::{
    CurrentProgressionSnapshot, FiniteProgressionPolicy, LevelThreshold,
    ProgressionCalculationError, ProgressionOperation, ProgressionRevisionContext,
    calculate_progression,
};
use oteryn_simulation_determinism::{ExactI64, NumericError, RoundingMode};

const NON_REFERENCE_THRESHOLDS: [LevelThreshold; 4] = [
    LevelThreshold {
        level: 3,
        minimum_experience: ExactI64::new(0),
    },
    LevelThreshold {
        level: 4,
        minimum_experience: ExactI64::new(100),
    },
    LevelThreshold {
        level: 5,
        minimum_experience: ExactI64::new(350),
    },
    LevelThreshold {
        level: 6,
        minimum_experience: ExactI64::new(900),
    },
];

fn context() -> ProgressionRevisionContext<&'static str> {
    ProgressionRevisionContext {
        profile: "synthetic-profile-r1",
        ruleset: "synthetic-ruleset-r2",
        content: "synthetic-content-r3",
        simulation: "sim-profile-r1",
        evidence: "NON_REFERENCE-structural-evidence-r1",
        declaration: "oteryn-declared-difference-r1",
    }
}

fn policy() -> FiniteProgressionPolicy<&'static str, 4> {
    FiniteProgressionPolicy {
        context: context(),
        policy_revision: "synthetic-policy-r4",
        reward_revision: "synthetic-reward-r5",
        death_policy_revision: "synthetic-death-r6",
        declared_difference_revision: "oteryn-declared-difference-r1",
        thresholds: NON_REFERENCE_THRESHOLDS,
        terminal_exclusive_experience: ExactI64::new(2_000),
        death_loss_numerator: 1,
        death_loss_denominator: 2,
        death_loss_rounding: RoundingMode::TowardZero,
    }
}

fn snapshot(level: u32, xp: i64) -> CurrentProgressionSnapshot<&'static str, &'static str> {
    CurrentProgressionSnapshot {
        level,
        total_experience: ExactI64::new(xp),
        skill_progression: "skills-bit-pattern",
        magic_progression: "magic-bit-pattern",
    }
}

fn award(amount: i64) -> ProgressionOperation<&'static str, &'static str> {
    ProgressionOperation::AwardExperience {
        source_occurrence: "synthetic-kill-occurrence",
        reward_revision: "synthetic-reward-r5",
        amount: ExactI64::new(amount),
    }
}

fn death() -> ProgressionOperation<&'static str, &'static str> {
    ProgressionOperation::ApplyDeathExperienceLoss {
        death_occurrence: "synthetic-death-occurrence",
        death_policy_revision: "synthetic-death-r6",
        declared_difference_revision: "oteryn-declared-difference-r1",
        regular_blessings: 0,
        promoted_with_current_premium: false,
    }
}

fn calculate(
    current: &CurrentProgressionSnapshot<&'static str, &'static str>,
    operation: &ProgressionOperation<&'static str, &'static str>,
) -> Result<
    oteryn_game_server::domain::progression::StagedProgressionOutcome<&'static str, &'static str>,
    ProgressionCalculationError,
> {
    calculate_progression(
        current,
        &context(),
        &"synthetic-policy-r4",
        operation,
        &policy(),
    )
}

#[test]
fn non_reference_award_below_threshold_keeps_level() -> Result<(), ProgressionCalculationError> {
    let result = calculate(&snapshot(3, 20), &award(70))?;
    assert_eq!((result.level_before, result.level_after), (3, 3));
    assert_eq!(result.experience_after, ExactI64::new(90));
    Ok(())
}

#[test]
fn non_reference_award_crosses_exact_threshold_and_multiple_thresholds()
-> Result<(), ProgressionCalculationError> {
    let exact = calculate(&snapshot(3, 20), &award(80))?;
    assert_eq!(exact.level_after, 4);
    let multiple = calculate(&snapshot(3, 20), &award(880))?;
    assert_eq!(multiple.level_after, 6);
    Ok(())
}

#[test]
fn declared_difference_death_can_delevel_and_preserves_skill_and_magic()
-> Result<(), ProgressionCalculationError> {
    let result = calculate(&snapshot(5, 400), &death())?;
    // D58 at level 5: (55/100) × 50 × (25 − 25 + 8) = 220; the synthetic
    // policy scale of one half loses 110.
    assert_eq!(result.death_experience_lost, ExactI64::new(110));
    assert_eq!(result.experience_after, ExactI64::new(290));
    assert_eq!(result.level_after, 4);
    assert_eq!(result.skill_progression, "skills-bit-pattern");
    assert_eq!(result.magic_progression, "magic-bit-pattern");
    Ok(())
}

#[test]
fn death_loss_depends_on_level_not_within_level_progress() -> Result<(), ProgressionCalculationError>
{
    let near_start = calculate(&snapshot(5, 360), &death())?;
    let near_end = calculate(&snapshot(5, 890), &death())?;
    assert_eq!(near_start.death_experience_lost, ExactI64::new(110));
    assert_eq!(
        near_start.death_experience_lost,
        near_end.death_experience_lost
    );
    Ok(())
}

#[test]
fn every_exact_context_revision_mismatch_fails_closed() {
    let current = snapshot(4, 120);
    let operation = award(1);
    for mutate in 0..6 {
        let mut mismatched = context();
        match mutate {
            0 => mismatched.profile = "wrong",
            1 => mismatched.ruleset = "wrong",
            2 => mismatched.content = "wrong",
            3 => mismatched.simulation = "wrong",
            4 => mismatched.evidence = "wrong",
            5 => mismatched.declaration = "wrong",
            _ => unreachable!(),
        }
        assert_eq!(
            calculate_progression(
                &current,
                &mismatched,
                &"synthetic-policy-r4",
                &operation,
                &policy()
            ),
            Err(ProgressionCalculationError::RevisionMismatch)
        );
    }
    assert_eq!(
        calculate_progression(&current, &context(), &"wrong", &operation, &policy()),
        Err(ProgressionCalculationError::RevisionMismatch)
    );
    assert_eq!(
        calculate(
            &current,
            &ProgressionOperation::AwardExperience {
                source_occurrence: "x",
                reward_revision: "wrong",
                amount: ExactI64::new(1),
            }
        ),
        Err(ProgressionCalculationError::RevisionMismatch)
    );
    assert_eq!(
        calculate(
            &current,
            &ProgressionOperation::ApplyDeathExperienceLoss {
                death_occurrence: "x",
                death_policy_revision: "synthetic-death-r6",
                declared_difference_revision: "wrong",
                regular_blessings: 0,
                promoted_with_current_premium: false,
            }
        ),
        Err(ProgressionCalculationError::RevisionMismatch)
    );
    assert_eq!(
        calculate(
            &current,
            &ProgressionOperation::ApplyDeathExperienceLoss {
                death_occurrence: "x",
                death_policy_revision: "wrong",
                declared_difference_revision: "oteryn-declared-difference-r1",
                regular_blessings: 0,
                promoted_with_current_premium: false,
            }
        ),
        Err(ProgressionCalculationError::RevisionMismatch)
    );
}

#[test]
fn policy_internal_declaration_revision_mismatch_fails_closed() {
    let current = snapshot(4, 120);
    let operation = award(1);
    let mut mismatched_policy = policy();
    mismatched_policy.declared_difference_revision = "split-declaration-r2";
    assert_eq!(
        calculate_progression(
            &current,
            &context(),
            &"synthetic-policy-r4",
            &operation,
            &mismatched_policy,
        ),
        Err(ProgressionCalculationError::RevisionMismatch)
    );
}

#[test]
fn missing_oracle_and_invalid_snapshot_fail_closed() {
    assert_eq!(
        calculate(&snapshot(6, 1_999), &award(1)),
        Err(ProgressionCalculationError::MissingThresholdOracle)
    );
    assert_eq!(
        calculate(&snapshot(4, 99), &award(1)),
        Err(ProgressionCalculationError::InvalidSnapshot)
    );
}

#[test]
fn checked_overflow_and_underflow_return_no_outcome() {
    let mut huge_policy = policy();
    huge_policy.thresholds = [
        LevelThreshold {
            level: 3,
            minimum_experience: ExactI64::new(0),
        },
        LevelThreshold {
            level: 4,
            minimum_experience: ExactI64::new(1),
        },
        LevelThreshold {
            level: 5,
            minimum_experience: ExactI64::new(2),
        },
        LevelThreshold {
            level: 6,
            minimum_experience: ExactI64::new(i64::MAX - 1),
        },
    ];
    huge_policy.terminal_exclusive_experience = ExactI64::new(i64::MAX);
    assert_eq!(
        calculate_progression(
            &snapshot(6, i64::MAX - 1),
            &context(),
            &"synthetic-policy-r4",
            &award(2),
            &huge_policy,
        ),
        Err(ProgressionCalculationError::Numeric(NumericError::Overflow))
    );

    let mut overflowing_level_policy = policy();
    overflowing_level_policy.thresholds[0].level = u32::MAX;
    assert_eq!(
        calculate_progression(
            &snapshot(3, 20),
            &context(),
            &"synthetic-policy-r4",
            &award(1),
            &overflowing_level_policy,
        ),
        Err(ProgressionCalculationError::Numeric(NumericError::Overflow))
    );

    // Level 3: (53/100) × 50 × 2 = 53, doubled to 106, capped at the 20 held.
    let mut total_loss = policy();
    total_loss.death_loss_numerator = 2;
    total_loss.death_loss_denominator = 1;
    let capped = calculate_progression(
        &snapshot(3, 20),
        &context(),
        &"synthetic-policy-r4",
        &death(),
        &total_loss,
    )
    .expect("capped death");
    assert_eq!(capped.death_experience_lost, ExactI64::new(20));
    assert_eq!(capped.experience_after, ExactI64::new(0));
    assert_eq!(capped.level_after, 3);
}

#[test]
fn unsupported_family_and_non_positive_award_are_rejected() {
    assert_eq!(
        calculate(
            &snapshot(4, 120),
            &ProgressionOperation::UnsupportedSkillOrProficiencyMutation
        ),
        Err(ProgressionCalculationError::UnsupportedOperation)
    );
    assert_eq!(
        calculate(&snapshot(4, 120), &award(0)),
        Err(ProgressionCalculationError::InvalidAward)
    );
}

#[test]
fn identical_scalar_inputs_are_deterministic() {
    let current = snapshot(4, 200);
    let operation = award(500);
    assert_eq!(
        calculate(&current, &operation),
        calculate(&current, &operation)
    );
}

/// Global experience threshold `50/3 × (L³ − 6L² + 17L − 12)`; `L = 0` gives
/// the virtual −200 that makes the level-one span 200 (D59).
fn global_threshold(level: i128) -> i128 {
    50 * (level * level * level - 6 * level * level + 17 * level - 12) / 3
}

const REFERENCE_LEVELS: usize = 501;

fn reference_policy() -> FiniteProgressionPolicy<&'static str, REFERENCE_LEVELS> {
    let mut thresholds = [LevelThreshold {
        level: 0,
        minimum_experience: ExactI64::new(0),
    }; REFERENCE_LEVELS];
    for (index, threshold) in thresholds.iter_mut().enumerate() {
        let level = u32::try_from(index + 1).expect("level");
        *threshold = LevelThreshold {
            level,
            minimum_experience: ExactI64::new(
                i64::try_from(global_threshold(i128::from(level))).expect("threshold"),
            ),
        };
    }
    FiniteProgressionPolicy {
        thresholds,
        terminal_exclusive_experience: ExactI64::new(
            i64::try_from(global_threshold(502)).expect("terminal"),
        ),
        death_loss_numerator: 1,
        death_loss_denominator: 1,
        death_loss_rounding: RoundingMode::Floor,
        ..policy_fields()
    }
}

fn policy_fields() -> FiniteProgressionPolicy<&'static str, REFERENCE_LEVELS> {
    let base = policy();
    FiniteProgressionPolicy {
        context: base.context,
        policy_revision: base.policy_revision,
        reward_revision: base.reward_revision,
        death_policy_revision: base.death_policy_revision,
        declared_difference_revision: base.declared_difference_revision,
        thresholds: [LevelThreshold {
            level: 0,
            minimum_experience: ExactI64::new(0),
        }; REFERENCE_LEVELS],
        terminal_exclusive_experience: base.terminal_exclusive_experience,
        death_loss_numerator: base.death_loss_numerator,
        death_loss_denominator: base.death_loss_denominator,
        death_loss_rounding: base.death_loss_rounding,
    }
}

fn reference_death(
    level: u32,
    experience: i64,
    blessings: u32,
    promoted: bool,
) -> Result<
    oteryn_game_server::domain::progression::StagedProgressionOutcome<&'static str, &'static str>,
    ProgressionCalculationError,
> {
    calculate_progression(
        &snapshot(level, experience),
        &context(),
        &"synthetic-policy-r4",
        &ProgressionOperation::ApplyDeathExperienceLoss {
            death_occurrence: "reference-death",
            death_policy_revision: "synthetic-death-r6",
            declared_difference_revision: "oteryn-declared-difference-r1",
            regular_blessings: blessings,
            promoted_with_current_premium: promoted,
        },
        &reference_policy(),
    )
}

#[test]
fn d58_loss_matches_the_global_span_form_at_every_required_level()
-> Result<(), ProgressionCalculationError> {
    for level in [1_u32, 23, 24, 100, 500] {
        let at = i128::from(level);
        let span = global_threshold(at) - global_threshold(at - 1);
        // One experience point below the next level: the most a character of
        // this level can hold, so the cap only binds where the loss exceeds it.
        let experience = i64::try_from(global_threshold(at + 1) - 1).expect("experience");
        for blessings in 0..=4_u32 {
            for promoted in [false, true] {
                let percent = 100 - 8 * i128::from(blessings) - if promoted { 30 } else { 0 };
                // Floor of (L+50)/100 × span × percent/100 (D68).
                let expected = ((at + 50) * span * percent / 10_000).min(i128::from(experience));
                let outcome = reference_death(level, experience, blessings, promoted)?;
                assert_eq!(
                    i128::from(outcome.death_experience_lost.get()),
                    expected,
                    "level {level}, {blessings} blessings, promoted {promoted}"
                );
                assert_eq!(
                    outcome.experience_after.get(),
                    experience - outcome.death_experience_lost.get()
                );
                assert_eq!(outcome.skill_progression, "skills-bit-pattern");
                assert_eq!(outcome.magic_progression, "magic-bit-pattern");
            }
        }
    }
    // Global: level 100 without reductions loses 1.5 × 50 × 9508.
    assert_eq!(
        reference_death(100, 16_000_000, 0, false)?.death_experience_lost,
        ExactI64::new(713_100)
    );
    Ok(())
}

#[test]
fn d58_loss_rounds_down_caps_at_zero_and_delevels() -> Result<(), ProgressionCalculationError> {
    // Level 24: 0.74 × 50 × 464 = 17168; with one blessing 0.92 × 17168 =
    // 15794.56, floored in the player's favour.
    assert_eq!(
        reference_death(24, 180_000, 1, false)?.death_experience_lost,
        ExactI64::new(15_794)
    );
    // Level 1 loses (51/100) × 50 × 4 = 102 of at most 99 held: capped at 0.
    let level_one = reference_death(1, 99, 0, false)?;
    assert_eq!(level_one.experience_after, ExactI64::new(0));
    assert_eq!(level_one.level_after, 1);
    // Level 9 at 6500 loses 0.59 × 50 × 44 = 1298 and drops to level 8.
    let delevel = reference_death(9, 6_500, 0, false)?;
    assert_eq!(delevel.experience_after, ExactI64::new(5_202));
    assert_eq!((delevel.level_before, delevel.level_after), (9, 8));
    Ok(())
}

#[test]
fn d58_reductions_beyond_the_whole_loss_fail_closed() {
    assert_eq!(
        reference_death(100, 16_000_000, 13, false).map(|_| ()),
        Err(ProgressionCalculationError::InvalidDeathReduction)
    );
    assert_eq!(
        reference_death(100, 16_000_000, 9, true).map(|_| ()),
        Err(ProgressionCalculationError::InvalidDeathReduction)
    );
    assert_eq!(
        reference_death(100, 16_000_000, u32::MAX, false).map(|_| ()),
        Err(ProgressionCalculationError::InvalidDeathReduction)
    );
    // Seven blessings and promotion reduce by 86%, which is admitted.
    assert!(reference_death(100, 16_000_000, 7, true).is_ok());
}
