use super::{
    AuthorityFenceEvidence, ChildLifecycle, ChildLifecycleBook, ChildOccurrenceRef,
    InteractionError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ForeignOwner {
    Foundation,
    Ability,
    Durability,
}

/// A delegated domain's own stable operation, transaction, or attempt identity.
///
/// Interaction stores and compares this opaque typed value but never creates a
/// process-wide operation identifier or interprets the domain-owned payload.
pub trait DelegatedOperation: Clone + Eq {
    fn owner(&self) -> ForeignOwner;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalResult {
    Pending,
    Rejected,
}

/// A typed proposal boundary to a domain that retains its own mutation authority.
pub trait ProposalAdapter {
    type Operation: DelegatedOperation;

    fn owner(&self) -> ForeignOwner;

    fn propose(
        &mut self,
        child: &ChildOccurrenceRef,
        operation: &Self::Operation,
        fence: &AuthorityFenceEvidence,
    ) -> ProposalResult;
}

#[derive(Debug, Default)]
pub struct InteractionDispatcher;

impl InteractionDispatcher {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    pub fn dispatch<A: ProposalAdapter>(
        &mut self,
        lifecycle: &mut ChildLifecycleBook<A::Operation>,
        child: &ChildOccurrenceRef,
        operation: &A::Operation,
        fence: &AuthorityFenceEvidence,
        adapter: &mut A,
    ) -> Result<ChildLifecycle, InteractionError> {
        if operation.owner() != adapter.owner() {
            return Err(InteractionError::ForeignOwnerMismatch);
        }
        let (state, should_propose) = lifecycle.begin(child, operation, fence)?;
        if !should_propose {
            return Ok(state);
        }
        lifecycle.apply_proposal(
            child,
            operation,
            fence,
            adapter.propose(child, operation, fence),
        )
    }
}
