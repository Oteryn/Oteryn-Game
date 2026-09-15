use super::occurrence::valid_atom;
use super::{AbilityError, MAX_RESOLVED_TARGETS, MAX_TARGET_CANDIDATES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalSource {
    Client,
    Ai,
    Script,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TargetId(String);

impl TargetId {
    pub fn new(value: &str) -> Result<Self, AbilityError> {
        if !valid_atom(value) {
            return Err(AbilityError::InvalidIdentifier);
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityIntent {
    proposal_source: ProposalSource,
    actor: String,
    candidate_count: usize,
    resolved_targets: Vec<TargetId>,
}

impl AbilityIntent {
    pub fn normalize(
        proposal_source: ProposalSource,
        actor: &str,
        candidates: &[&str],
    ) -> Result<Self, AbilityError> {
        Self::resolve(proposal_source, actor, candidates, candidates)
    }

    pub fn resolve(
        proposal_source: ProposalSource,
        actor: &str,
        candidates: &[&str],
        resolved: &[&str],
    ) -> Result<Self, AbilityError> {
        if !valid_atom(actor) {
            return Err(AbilityError::InvalidIdentifier);
        }
        if candidates.is_empty() {
            return Err(AbilityError::NoTargetCandidates);
        }
        if candidates.len() > MAX_TARGET_CANDIDATES {
            return Err(AbilityError::TooManyTargetCandidates);
        }
        if resolved.is_empty() {
            return Err(AbilityError::NoTargetCandidates);
        }
        if resolved.len() > MAX_RESOLVED_TARGETS {
            return Err(AbilityError::TooManyResolvedTargets);
        }

        if candidates.iter().any(|candidate| !valid_atom(candidate)) {
            return Err(AbilityError::InvalidIdentifier);
        }
        let mut resolved_targets = Vec::with_capacity(resolved.len());
        for target in resolved {
            resolved_targets.push(TargetId::new(target)?);
        }
        resolved_targets.sort();
        resolved_targets.dedup();
        if resolved_targets.len() > MAX_RESOLVED_TARGETS {
            return Err(AbilityError::TooManyResolvedTargets);
        }

        Ok(Self {
            proposal_source,
            actor: actor.to_owned(),
            candidate_count: candidates.len(),
            resolved_targets,
        })
    }

    #[must_use]
    pub const fn proposal_source(&self) -> ProposalSource {
        self.proposal_source
    }

    #[must_use]
    pub fn actor(&self) -> &str {
        &self.actor
    }

    #[must_use]
    pub const fn candidate_count(&self) -> usize {
        self.candidate_count
    }

    #[must_use]
    pub fn resolved_targets(&self) -> &[TargetId] {
        &self.resolved_targets
    }
}

pub struct ClientAbilityAdapter;

impl ClientAbilityAdapter {
    pub fn normalize(actor: &str, candidates: &[&str]) -> Result<AbilityIntent, AbilityError> {
        AbilityIntent::normalize(ProposalSource::Client, actor, candidates)
    }
}

pub struct AiAbilityAdapter;

impl AiAbilityAdapter {
    pub fn normalize(actor: &str, candidates: &[&str]) -> Result<AbilityIntent, AbilityError> {
        AbilityIntent::normalize(ProposalSource::Ai, actor, candidates)
    }
}

pub struct ScriptAbilityAdapter;

impl ScriptAbilityAdapter {
    pub fn normalize(actor: &str, candidates: &[&str]) -> Result<AbilityIntent, AbilityError> {
        AbilityIntent::normalize(ProposalSource::Script, actor, candidates)
    }
}
