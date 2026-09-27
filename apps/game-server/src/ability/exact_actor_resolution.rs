// One exact actor proposal, one current-owner lookup, at most one result.
// No collection, geometry, identity conversion or production composition.

use super::AbilityOccurrence;
use crate::foundation::{CurrentOwnerExactActorLookup, ExactActorRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExactActorSource {
    Client,
    Ai,
}

/// An untrusted proposal. The actor reference itself must already have been
/// issued by the Channel owner; its origin never grants current authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactActorProposal {
    source: ExactActorSource,
    target: ExactActorRef,
}

impl ExactActorProposal {
    pub(crate) fn client(target: ExactActorRef) -> Self {
        Self {
            source: ExactActorSource::Client,
            target,
        }
    }

    pub(crate) fn ai(target: ExactActorRef) -> Self {
        Self {
            source: ExactActorSource::Ai,
            target,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExactActorResolutionError {
    NotCurrentActor,
    OccurrenceSubstitution,
    RevisionSubstitution,
    TargetSubstitution,
    SourceSubstitution,
}

/// Constructed only by the resolver after checking the current owner's slot.
/// The occurrence retains every semantic revision and the exact actor identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedExactActor {
    occurrence: AbilityOccurrence,
    target: ExactActorRef,
    source: ExactActorSource,
}

impl ResolvedExactActor {
    pub(crate) fn occurrence(&self) -> &AbilityOccurrence {
        &self.occurrence
    }

    pub(crate) fn target(&self) -> ExactActorRef {
        self.target
    }

    pub(crate) fn source(&self) -> ExactActorSource {
        self.source
    }

    pub(crate) fn candidate_count(&self) -> usize {
        1
    }

    pub(crate) fn resolved_count(&self) -> usize {
        1
    }

    /// Stateless retry comparison. A current-owner lookup is still required
    /// for any later mutation; this snapshot cannot renew actor authority.
    pub(crate) fn reconcile(
        &self,
        occurrence: &AbilityOccurrence,
        proposal: ExactActorProposal,
    ) -> Result<(), ExactActorResolutionError> {
        if self.occurrence.id() != occurrence.id() {
            return Err(ExactActorResolutionError::OccurrenceSubstitution);
        }
        if self.occurrence.revisions() != occurrence.revisions() {
            return Err(ExactActorResolutionError::RevisionSubstitution);
        }
        if self.target != proposal.target {
            return Err(ExactActorResolutionError::TargetSubstitution);
        }
        if self.source != proposal.source {
            return Err(ExactActorResolutionError::SourceSubstitution);
        }
        Ok(())
    }
}

pub(crate) fn resolve_exact_actor(
    owner: &CurrentOwnerExactActorLookup<'_>,
    occurrence: &AbilityOccurrence,
    proposal: ExactActorProposal,
) -> Result<ResolvedExactActor, ExactActorResolutionError> {
    if !owner.contains(proposal.target) {
        return Err(ExactActorResolutionError::NotCurrentActor);
    }
    Ok(ResolvedExactActor {
        occurrence: occurrence.clone(),
        target: proposal.target,
        source: proposal.source,
    })
}
