//! Compatibility exports for the shared interaction semantic core.

#![forbid(unsafe_code)]

pub use crate::domain::interaction::{
    AuthorityFenceEvidence, ChildLifecycle, ChildLifecycleBook, ChildOccurrenceRef,
    DelegatedOperation, ForeignOwner, InteractionDispatcher, InteractionError, InteractionPlan,
    ProposalAdapter, ProposalResult, ReconciliationOutcome, RetainedChildLifecycles,
    RngDecisionRef, RootSourceOccurrenceRef, SelectedChildWork, SemanticRevisionContext,
    TriggerRegistration,
};
