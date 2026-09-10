//! Pure, persistence-neutral Character experience projection.

use oteryn_simulation_determinism::{ExactI64, FixedScale, NumericError, RoundingMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressionRevisionContext<R> {
    pub profile: R,
    pub ruleset: R,
    pub content: R,
    pub simulation: R,
    pub evidence: R,
    pub declaration: R,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentProgressionSnapshot<S, M> {
    pub level: u32,
    pub total_experience: ExactI64,
    pub skill_progression: S,
    pub magic_progression: M,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelThreshold {
    pub level: u32,
    pub minimum_experience: ExactI64,
}

/// A finite oracle. `terminal_exclusive_experience` bounds the final listed level;
/// values outside the table fail closed rather than extrapolating a formula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteProgressionPolicy<R, const N: usize> {
    pub context: ProgressionRevisionContext<R>,
    pub policy_revision: R,
    pub reward_revision: R,
    pub death_policy_revision: R,
    pub declared_difference_revision: R,
    pub thresholds: [LevelThreshold; N],
    pub terminal_exclusive_experience: ExactI64,
    pub death_loss_numerator: i64,
    pub death_loss_denominator: i64,
    pub death_loss_rounding: RoundingMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressionOperation<O, R> {
    AwardExperience {
        source_occurrence: O,
        reward_revision: R,
        amount: ExactI64,
    },
    ApplyDeathExperienceLoss {
        death_occurrence: O,
        death_policy_revision: R,
        declared_difference_revision: R,
    },
    /// Closed marker allowing callers to reject skill/proficiency families without
    /// translating them into an arbitrary signed experience delta.
    UnsupportedSkillOrProficiencyMutation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedProgressionOutcome<S, M> {
    pub level_before: u32,
    pub level_after: u32,
    pub experience_before: ExactI64,
    pub experience_after: ExactI64,
    pub experience_awarded: ExactI64,
    pub death_experience_lost: ExactI64,
    pub skill_progression: S,
    pub magic_progression: M,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressionCalculationError {
    RevisionMismatch,
    MissingThresholdOracle,
    InvalidPolicy,
    InvalidSnapshot,
    InvalidAward,
    ExperienceUnderflow,
    Numeric(NumericError),
    UnsupportedOperation,
}

impl From<NumericError> for ProgressionCalculationError {
    fn from(value: NumericError) -> Self {
        Self::Numeric(value)
    }
}

pub fn calculate_progression<O, R, S, M, const N: usize>(
    snapshot: &CurrentProgressionSnapshot<S, M>,
    operation_context: &ProgressionRevisionContext<R>,
    operation_policy_revision: &R,
    operation: &ProgressionOperation<O, R>,
    policy: &FiniteProgressionPolicy<R, N>,
) -> Result<StagedProgressionOutcome<S, M>, ProgressionCalculationError>
where
    R: PartialEq,
    S: Clone,
    M: Clone,
{
    validate_policy(policy)?;
    if operation_context != &policy.context || operation_policy_revision != &policy.policy_revision
    {
        return Err(ProgressionCalculationError::RevisionMismatch);
    }
    let projected_current = project_level(snapshot.total_experience, policy)?;
    if snapshot.total_experience.get() < 0 || projected_current != snapshot.level {
        return Err(ProgressionCalculationError::InvalidSnapshot);
    }

    let zero = ExactI64::new(0);
    let (experience_after, awarded, lost) = match operation {
        ProgressionOperation::AwardExperience {
            reward_revision,
            amount,
            ..
        } => {
            if reward_revision != &policy.reward_revision {
                return Err(ProgressionCalculationError::RevisionMismatch);
            }
            if amount.get() <= 0 {
                return Err(ProgressionCalculationError::InvalidAward);
            }
            (
                snapshot.total_experience.checked_add(*amount)?,
                *amount,
                zero,
            )
        }
        ProgressionOperation::ApplyDeathExperienceLoss {
            death_policy_revision,
            declared_difference_revision,
            ..
        } => {
            if death_policy_revision != &policy.death_policy_revision
                || declared_difference_revision != &policy.declared_difference_revision
            {
                return Err(ProgressionCalculationError::RevisionMismatch);
            }
            let span = level_span(snapshot.level, policy)?;
            let lost = FixedScale::new(span.get(), 0)?
                .checked_mul_ratio(
                    policy.death_loss_numerator,
                    policy.death_loss_denominator,
                    policy.death_loss_rounding,
                )?
                .raw();
            if lost < 0 {
                return Err(ProgressionCalculationError::InvalidPolicy);
            }
            let after = snapshot.total_experience.checked_sub(ExactI64::new(lost))?;
            if after.get() < 0 {
                return Err(ProgressionCalculationError::ExperienceUnderflow);
            }
            (after, zero, ExactI64::new(lost))
        }
        ProgressionOperation::UnsupportedSkillOrProficiencyMutation => {
            return Err(ProgressionCalculationError::UnsupportedOperation);
        }
    };

    let level_after = project_level(experience_after, policy)?;
    Ok(StagedProgressionOutcome {
        level_before: snapshot.level,
        level_after,
        experience_before: snapshot.total_experience,
        experience_after,
        experience_awarded: awarded,
        death_experience_lost: lost,
        skill_progression: snapshot.skill_progression.clone(),
        magic_progression: snapshot.magic_progression.clone(),
    })
}

fn validate_policy<R, const N: usize>(
    policy: &FiniteProgressionPolicy<R, N>,
) -> Result<(), ProgressionCalculationError> {
    if N < 2 || policy.death_loss_numerator < 0 || policy.death_loss_denominator <= 0 {
        return Err(ProgressionCalculationError::InvalidPolicy);
    }
    for pair in policy.thresholds.windows(2) {
        let expected_level = pair[0]
            .level
            .checked_add(1)
            .ok_or(ProgressionCalculationError::Numeric(NumericError::Overflow))?;
        if pair[1].level != expected_level
            || pair[0].minimum_experience.get() < 0
            || pair[1].minimum_experience.get() <= pair[0].minimum_experience.get()
        {
            return Err(ProgressionCalculationError::InvalidPolicy);
        }
        pair[1]
            .minimum_experience
            .checked_sub(pair[0].minimum_experience)?;
    }
    let last = policy.thresholds[N - 1].minimum_experience;
    if policy.terminal_exclusive_experience.get() <= last.get() {
        return Err(ProgressionCalculationError::InvalidPolicy);
    }
    policy.terminal_exclusive_experience.checked_sub(last)?;
    Ok(())
}

fn project_level<R, const N: usize>(
    experience: ExactI64,
    policy: &FiniteProgressionPolicy<R, N>,
) -> Result<u32, ProgressionCalculationError> {
    if experience.get() < policy.thresholds[0].minimum_experience.get()
        || experience.get() >= policy.terminal_exclusive_experience.get()
    {
        return Err(ProgressionCalculationError::MissingThresholdOracle);
    }
    policy
        .thresholds
        .iter()
        .rev()
        .find(|entry| experience.get() >= entry.minimum_experience.get())
        .map(|entry| entry.level)
        .ok_or(ProgressionCalculationError::MissingThresholdOracle)
}

fn level_span<R, const N: usize>(
    level: u32,
    policy: &FiniteProgressionPolicy<R, N>,
) -> Result<ExactI64, ProgressionCalculationError> {
    let index = policy
        .thresholds
        .iter()
        .position(|entry| entry.level == level)
        .ok_or(ProgressionCalculationError::MissingThresholdOracle)?;
    let lower = policy.thresholds[index].minimum_experience;
    let upper = policy
        .thresholds
        .get(index + 1)
        .map_or(policy.terminal_exclusive_experience, |entry| {
            entry.minimum_experience
        });
    Ok(upper.checked_sub(lower)?)
}
