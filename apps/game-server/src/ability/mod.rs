//! Bounded, fixture-only authoritative ability occurrence pipeline.
//!
//! This module deliberately models only immediate typed damage and healing fixtures. It does not
//! select Reference formulas, introduce protocol identifiers, or compose the game-server root.

#![forbid(unsafe_code)]

mod commit;
mod effects;
mod intent;
mod occurrence;
mod plan;

pub use commit::{AbilityEngine, CommitReceipt};
pub use effects::Effect;
pub use intent::{
    AbilityIntent, AiAbilityAdapter, ClientAbilityAdapter, ProposalSource, ScriptAbilityAdapter,
    TargetId,
};
pub use occurrence::{AbilityOccurrence, AbilityOccurrenceId, RevisionSet};
pub use plan::{CalculationStage, CommitGroup, CommitGroupMode, EffectPlan, SubOccurrenceRef};

use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const MAX_TARGET_CANDIDATES: usize = 2;
pub const MAX_RESOLVED_TARGETS: usize = 2;
pub const MAX_EFFECT_PLAN_ENTRIES: usize = 2;
pub const MAX_EFFECT_PLAN_BYTES: usize = 4_096;
pub const MAX_CALCULATION_STAGES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityError {
    InvalidIdentifier,
    MissingRevision,
    NoTargetCandidates,
    TooManyTargetCandidates,
    TooManyResolvedTargets,
    TooManyEffectPlanEntries,
    EffectPlanTooLarge,
    RetainedByteOverflow,
    TooManyCalculationStages,
    DuplicateCalculationStage,
    EmptyEffectPlan,
    TargetOutsideResolvedSet,
    InvalidMagnitude,
    OccurrenceRevisionConflict,
    OccurrencePlanConflict,
    MissingSubOccurrence,
    NumericOverflow,
}

impl Display for AbilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidIdentifier => "ability fixture identifier is invalid",
            Self::MissingRevision => "ability occurrence requires every semantic revision",
            Self::NoTargetCandidates => "immediate fixture requires a target candidate",
            Self::TooManyTargetCandidates => "target candidate limit exceeded",
            Self::TooManyResolvedTargets => "resolved target limit exceeded",
            Self::TooManyEffectPlanEntries => "effect plan entry limit exceeded",
            Self::EffectPlanTooLarge => "effect plan byte limit exceeded",
            Self::RetainedByteOverflow => "effect plan retained-byte accounting overflowed",
            Self::TooManyCalculationStages => "calculation stage limit exceeded",
            Self::DuplicateCalculationStage => "calculation stages must have unique identities",
            Self::EmptyEffectPlan => "immediate fixture requires one typed effect",
            Self::TargetOutsideResolvedSet => "effect target is not in the resolved target set",
            Self::InvalidMagnitude => "damage or healing magnitude must be positive",
            Self::OccurrenceRevisionConflict => {
                "a retry may not reinterpret an occurrence under another revision"
            }
            Self::OccurrencePlanConflict => "a retry may not replace a committed effect plan",
            Self::MissingSubOccurrence => "effect plan lacks an expected sub-occurrence",
            Self::NumericOverflow => "fixture health transition overflowed",
        })
    }
}

impl Error for AbilityError {}

#[cfg(test)]
mod tests;
