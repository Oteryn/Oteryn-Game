//! Bounded deterministic interaction planning and reconciliation.
//!
//! This module owns only interaction occurrence identity, planning and the
//! proposal/reconciliation boundary. Foreign domains retain mutation authority.

#![forbid(unsafe_code)]

mod dispatch;
mod identity;
mod lifecycle;
mod plan;

pub use dispatch::{
    DelegatedOperation, ForeignOwner, InteractionDispatcher, ProposalAdapter, ProposalResult,
};
pub use identity::{
    AuthorityFenceEvidence, ChildOccurrenceRef, RngDecisionRef, RootSourceOccurrenceRef,
    SemanticRevisionContext,
};
pub use lifecycle::{ChildLifecycle, ChildLifecycleBook, ReconciliationOutcome};
pub use plan::{InteractionPlan, RetainedChildLifecycles, SelectedChildWork, TriggerRegistration};

use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionError {
    EmptySemanticKey,
    InvalidAuthorityFence,
    CapacityExceeded {
        resource: &'static str,
        limit: usize,
        observed: usize,
    },
    CountOverflow,
    DuplicateChild,
    DuplicateTriggerRegistration,
    MismatchedRoot,
    UnknownChild,
    StaleOwnerOperation,
    StaleAuthorityFence,
    ForeignOwnerMismatch,
    MissingRetainedLifecycle,
    UnregisteredSelectedChild,
}

impl Display for InteractionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySemanticKey => {
                formatter.write_str("interaction semantic keys must be non-empty")
            }
            Self::InvalidAuthorityFence => {
                formatter.write_str("interaction authority fence fields must be non-zero")
            }
            Self::CapacityExceeded {
                resource,
                limit,
                observed,
            } => write!(
                formatter,
                "interaction {resource} capacity exceeded: observed {observed}, limit {limit}"
            ),
            Self::CountOverflow => formatter.write_str("interaction count overflow"),
            Self::DuplicateChild => {
                formatter.write_str("interaction child occurrence is duplicated")
            }
            Self::DuplicateTriggerRegistration => {
                formatter.write_str("interaction trigger registration is duplicated")
            }
            Self::MismatchedRoot => {
                formatter.write_str("interaction child belongs to another root occurrence")
            }
            Self::UnknownChild => {
                formatter.write_str("interaction child is absent from the accepted plan")
            }
            Self::StaleOwnerOperation => {
                formatter.write_str("interaction owner operation does not match the pending child")
            }
            Self::StaleAuthorityFence => {
                formatter.write_str("interaction authority fence does not match the pending child")
            }
            Self::ForeignOwnerMismatch => {
                formatter.write_str("interaction adapter does not own the supplied operation")
            }
            Self::MissingRetainedLifecycle => {
                formatter.write_str("interaction plan child has no retained lifecycle entry")
            }
            Self::UnregisteredSelectedChild => {
                formatter.write_str("interaction selected child has no trigger registration")
            }
        }
    }
}

impl Error for InteractionError {}

#[cfg(test)]
mod tests;
