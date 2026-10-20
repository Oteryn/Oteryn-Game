use super::{
    AuthorityFenceEvidence, ChildOccurrenceRef, DelegatedOperation, InteractionError,
    InteractionPlan, ProposalResult,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildLifecycle {
    Unstarted,
    Pending,
    Committed,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationOutcome {
    Ambiguous,
    Committed,
    Rejected,
    CancellationRequested,
    CancelledWithRetirementProof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LifecycleEntry<O> {
    state: ChildLifecycle,
    operation: Option<O>,
    fence: Option<AuthorityFenceEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildLifecycleBook<O: DelegatedOperation> {
    entries: BTreeMap<ChildOccurrenceRef, LifecycleEntry<O>>,
}

impl<O: DelegatedOperation> ChildLifecycleBook<O> {
    #[must_use]
    pub fn from_plan(plan: &InteractionPlan) -> Self {
        let entries = plan
            .children()
            .iter()
            .cloned()
            .map(|child| {
                (
                    child,
                    LifecycleEntry {
                        state: ChildLifecycle::Unstarted,
                        operation: None,
                        fence: None,
                    },
                )
            })
            .collect();
        Self { entries }
    }

    pub fn state(&self, child: &ChildOccurrenceRef) -> Result<ChildLifecycle, InteractionError> {
        self.entries
            .get(child)
            .map(|entry| entry.state)
            .ok_or(InteractionError::UnknownChild)
    }

    pub fn reconcile(
        &mut self,
        child: &ChildOccurrenceRef,
        operation: &O,
        fence: &AuthorityFenceEvidence,
        outcome: ReconciliationOutcome,
    ) -> Result<ChildLifecycle, InteractionError> {
        let entry = self
            .entries
            .get_mut(child)
            .ok_or(InteractionError::UnknownChild)?;
        ensure_same_pending_operation(entry, operation)?;
        ensure_same_authority_fence(entry, fence)?;
        match entry.state {
            ChildLifecycle::Pending => match outcome {
                ReconciliationOutcome::Committed => entry.state = ChildLifecycle::Committed,
                ReconciliationOutcome::Rejected
                | ReconciliationOutcome::CancelledWithRetirementProof => {
                    entry.state = ChildLifecycle::Rejected;
                }
                ReconciliationOutcome::Ambiguous | ReconciliationOutcome::CancellationRequested => {
                }
            },
            ChildLifecycle::Unstarted | ChildLifecycle::Committed | ChildLifecycle::Rejected => {}
        }
        Ok(entry.state)
    }

    pub(crate) fn begin(
        &mut self,
        child: &ChildOccurrenceRef,
        operation: &O,
        fence: &AuthorityFenceEvidence,
    ) -> Result<(ChildLifecycle, bool), InteractionError> {
        let entry = self
            .entries
            .get_mut(child)
            .ok_or(InteractionError::UnknownChild)?;
        match entry.state {
            ChildLifecycle::Unstarted => {
                entry.state = ChildLifecycle::Pending;
                entry.operation = Some(operation.clone());
                entry.fence = Some(*fence);
                Ok((ChildLifecycle::Pending, true))
            }
            ChildLifecycle::Pending => {
                ensure_same_pending_operation(entry, operation)?;
                ensure_same_authority_fence(entry, fence)?;
                Ok((ChildLifecycle::Pending, false))
            }
            ChildLifecycle::Committed | ChildLifecycle::Rejected => Ok((entry.state, false)),
        }
    }

    pub(crate) fn apply_proposal(
        &mut self,
        child: &ChildOccurrenceRef,
        operation: &O,
        fence: &AuthorityFenceEvidence,
        result: ProposalResult,
    ) -> Result<ChildLifecycle, InteractionError> {
        let entry = self
            .entries
            .get_mut(child)
            .ok_or(InteractionError::UnknownChild)?;
        if entry.state != ChildLifecycle::Pending {
            return Err(InteractionError::StaleOwnerOperation);
        }
        ensure_same_pending_operation(entry, operation)?;
        ensure_same_authority_fence(entry, fence)?;
        if result == ProposalResult::Rejected {
            entry.state = ChildLifecycle::Rejected;
        }
        Ok(entry.state)
    }
}

fn ensure_same_pending_operation<O: DelegatedOperation>(
    entry: &LifecycleEntry<O>,
    operation: &O,
) -> Result<(), InteractionError> {
    if entry.operation.as_ref() != Some(operation) {
        return Err(InteractionError::StaleOwnerOperation);
    }
    Ok(())
}

fn ensure_same_authority_fence<O>(
    entry: &LifecycleEntry<O>,
    fence: &AuthorityFenceEvidence,
) -> Result<(), InteractionError> {
    if entry.fence != Some(*fence) {
        return Err(InteractionError::StaleAuthorityFence);
    }
    Ok(())
}
