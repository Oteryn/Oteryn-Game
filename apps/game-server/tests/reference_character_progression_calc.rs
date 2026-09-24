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
    // Full synthetic level-5 span is 900 - 350 = 550; one half loses 275.
    assert_eq!(result.death_experience_lost, ExactI64::new(275));
    assert_eq!(result.experience_after, ExactI64::new(125));
    assert_eq!(result.level_after, 4);
    assert_eq!(result.skill_progression, "skills-bit-pattern");
    assert_eq!(result.magic_progression, "magic-bit-pattern");
    Ok(())
}

#[test]
fn declared_difference_death_uses_full_level_span_not_within_level_progress()
-> Result<(), ProgressionCalculationError> {
    let near_start = calculate(&snapshot(5, 360), &death())?;
    let near_end = calculate(&snapshot(5, 890), &death())?;
    assert_eq!(near_start.death_experience_lost, ExactI64::new(275));
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

    let mut total_loss = policy();
    total_loss.death_loss_numerator = 2;
    total_loss.death_loss_denominator = 1;
    assert_eq!(
        calculate_progression(
            &snapshot(3, 20),
            &context(),
            &"synthetic-policy-r4",
            &death(),
            &total_loss
        ),
        Err(ProgressionCalculationError::ExperienceUnderflow)
    );
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
