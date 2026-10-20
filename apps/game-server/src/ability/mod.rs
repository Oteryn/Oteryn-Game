//! Bounded, fixture-only authoritative ability occurrence pipeline.
//!
//! This module deliberately models only immediate typed damage and healing fixtures. It does not
//! select Reference formulas, introduce protocol identifiers, or compose the game-server root.

#![forbid(unsafe_code)]
// This private, unactivated module keeps its established fixture facade for
// standalone tests while the production composition remains closed.
#![allow(unused_imports)]

pub(crate) mod commit;
#[allow(dead_code)]
pub(crate) use crate::foundation::condition;
#[allow(dead_code)]
pub(crate) mod creature_bite;
mod effects;
#[allow(dead_code)]
pub(crate) mod exact_actor_resolution;
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

pub use crate::foundation::ability_data::{
    AbilityError, MAX_CALCULATION_STAGES, MAX_EFFECT_PLAN_BYTES, MAX_EFFECT_PLAN_ENTRIES,
    MAX_RESOLVED_TARGETS, MAX_TARGET_CANDIDATES,
};

#[cfg(test)]
mod charm_desc_fence_gate_tests;
#[cfg(test)]
mod tests;

pub(crate) mod field_condition;
