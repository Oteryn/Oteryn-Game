//! Pure-local, deterministic AI bootstrap evidence.
//!
//! This module deliberately exposes only immutable snapshots, normalized decisions and path
//! proposals. It has no Movement, Ability, Interaction, persistence, value or reward mutation
//! surface. The composition root may elect to include it only after coordinator integration.

#![forbid(unsafe_code)]

mod path_proposal;
mod perception;
mod resolution;
mod snapshot;

#[cfg(test)]
mod tests;

pub use path_proposal::{PathProposal, PathRequest, RouteStep, build_path_proposal};
pub use perception::{Candidate, CandidateId, Perception, canonicalize_perception};
pub use resolution::{Decision, DecisionUnit, resolve};
pub use snapshot::{ActorId, AiProvenance, AiProvenanceInput, AiSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLimit {
    ActiveActors,
    AuthoredUnits,
    EvaluationWork,
    PerceptionCandidates,
    PathRequestsPerActor,
    PathSearchWork,
    RouteSteps,
    RouteBytes,
}

impl ResourceLimit {
    pub const ALL: [Self; 8] = [
        Self::ActiveActors,
        Self::AuthoredUnits,
        Self::EvaluationWork,
        Self::PerceptionCandidates,
        Self::PathRequestsPerActor,
        Self::PathSearchWork,
        Self::RouteSteps,
        Self::RouteBytes,
    ];

    #[must_use]
    pub const fn maximum(self) -> usize {
        match self {
            Self::ActiveActors => 256,
            Self::AuthoredUnits => 4,
            Self::EvaluationWork => 8,
            Self::PerceptionCandidates => 64,
            Self::PathRequestsPerActor => 2,
            Self::PathSearchWork => 1_024,
            Self::RouteSteps => 128,
            Self::RouteBytes => 4_096,
        }
    }

    pub fn admit(self, retained: usize, additional: usize) -> Result<usize, AiError> {
        let total = retained
            .checked_add(additional)
            .ok_or(AiError::ArithmeticOverflow(self))?;
        if total > self.maximum() {
            return Err(AiError::CapacityExceeded(self));
        }
        Ok(total)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiError {
    ArithmeticOverflow(ResourceLimit),
    BootstrapPathRequestLimit,
    CapacityExceeded(ResourceLimit),
    ExcludedDimension,
    EvaluationExhausted,
    InvalidInput,
    StaleProvenance,
}

/// Only these local operations can be admitted by the bootstrap. Every other lifecycle or
/// cross-domain dimension is explicitly reject-only until a separately accepted owner slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapOperation {
    Resolve,
    PathProposal,
    ControlledActor,
    Diagnostics,
    Memory,
    RepathRetry,
    Script,
    Spawn,
    Timer,
}

impl BootstrapOperation {
    pub const fn reject(self) -> Result<(), AiError> {
        match self {
            Self::Resolve | Self::PathProposal => Ok(()),
            Self::ControlledActor
            | Self::Diagnostics
            | Self::Memory
            | Self::RepathRetry
            | Self::Script
            | Self::Spawn
            | Self::Timer => Err(AiError::ExcludedDimension),
        }
    }
}
