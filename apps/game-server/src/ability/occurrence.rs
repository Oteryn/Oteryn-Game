use super::{AbilityError, MAX_EFFECT_PLAN_BYTES};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbilityOccurrenceId(String);

impl AbilityOccurrenceId {
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
pub struct RevisionSet {
    ruleset: String,
    content: String,
    world_policy: String,
    formula: String,
    simulation: String,
}

impl RevisionSet {
    pub fn new(
        ruleset: &str,
        content: &str,
        world_policy: &str,
        formula: &str,
        simulation: &str,
    ) -> Result<Self, AbilityError> {
        let revisions = [ruleset, content, world_policy, formula, simulation];
        if revisions.iter().any(|revision| !valid_atom(revision)) {
            return Err(AbilityError::MissingRevision);
        }
        Ok(Self {
            ruleset: ruleset.to_owned(),
            content: content.to_owned(),
            world_policy: world_policy.to_owned(),
            formula: formula.to_owned(),
            simulation: simulation.to_owned(),
        })
    }

    #[must_use]
    pub fn formula(&self) -> &str {
        &self.formula
    }

    #[must_use]
    pub fn simulation(&self) -> &str {
        &self.simulation
    }

    #[must_use]
    pub fn ruleset(&self) -> &str {
        &self.ruleset
    }

    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    #[must_use]
    pub fn world_policy(&self) -> &str {
        &self.world_policy
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityOccurrence {
    id: AbilityOccurrenceId,
    revisions: RevisionSet,
}

impl AbilityOccurrence {
    pub fn new(id: &str, revisions: RevisionSet) -> Result<Self, AbilityError> {
        Ok(Self {
            id: AbilityOccurrenceId::new(id)?,
            revisions,
        })
    }

    #[must_use]
    pub fn id(&self) -> &AbilityOccurrenceId {
        &self.id
    }

    #[must_use]
    pub fn revisions(&self) -> &RevisionSet {
        &self.revisions
    }
}

pub(crate) fn valid_atom(value: &str) -> bool {
    !value.is_empty()
        && value.is_ascii()
        && value.len() <= MAX_EFFECT_PLAN_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'.' | b'_' | b'-' | b'/')
        })
}
