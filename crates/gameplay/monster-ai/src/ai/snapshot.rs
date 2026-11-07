use super::{AiError, ResourceLimit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActorId(u64);

impl ActorId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Immutable owner, actor and exact interpretation revisions for one AI-local occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiProvenance {
    scope_id: u64,
    scope_generation: u64,
    actor_generation: u64,
    behavior_revision: u64,
    content_revision: u64,
    navigation_revision: u64,
    ruleset_revision: u64,
    determinism_profile_revision: u64,
}

/// Exact immutable values which identify an AI-local interpretation occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiProvenanceInput {
    pub scope_id: u64,
    pub scope_generation: u64,
    pub actor_generation: u64,
    pub behavior_revision: u64,
    pub content_revision: u64,
    pub navigation_revision: u64,
    pub ruleset_revision: u64,
    pub determinism_profile_revision: u64,
}

impl AiProvenance {
    #[must_use]
    pub const fn new(input: AiProvenanceInput) -> Self {
        Self {
            scope_id: input.scope_id,
            scope_generation: input.scope_generation,
            actor_generation: input.actor_generation,
            behavior_revision: input.behavior_revision,
            content_revision: input.content_revision,
            navigation_revision: input.navigation_revision,
            ruleset_revision: input.ruleset_revision,
            determinism_profile_revision: input.determinism_profile_revision,
        }
    }

    #[must_use]
    pub const fn with_actor_generation(mut self, actor_generation: u64) -> Self {
        self.actor_generation = actor_generation;
        self
    }

    #[must_use]
    pub const fn with_scope_generation(mut self, scope_generation: u64) -> Self {
        self.scope_generation = scope_generation;
        self
    }

    #[must_use]
    pub const fn with_content_revision(mut self, content_revision: u64) -> Self {
        self.content_revision = content_revision;
        self
    }

    #[must_use]
    pub const fn with_navigation_revision(mut self, navigation_revision: u64) -> Self {
        self.navigation_revision = navigation_revision;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiSnapshot {
    provenance: AiProvenance,
    active_actors: Vec<ActorId>,
}

impl AiSnapshot {
    pub fn new(provenance: AiProvenance, active_actors: &[ActorId]) -> Result<Self, AiError> {
        ResourceLimit::ActiveActors.admit(0, active_actors.len())?;
        let mut canonical_actors = active_actors.to_vec();
        canonical_actors.sort_unstable();
        if canonical_actors
            .windows(2)
            .any(|window| window[0] == window[1])
        {
            return Err(AiError::InvalidInput);
        }
        Ok(Self {
            provenance,
            active_actors: canonical_actors,
        })
    }

    #[must_use]
    pub const fn provenance(&self) -> AiProvenance {
        self.provenance
    }

    #[must_use]
    pub fn active_actors(&self) -> &[ActorId] {
        &self.active_actors
    }

    pub fn require_current(&self, current: AiProvenance) -> Result<(), AiError> {
        if self.provenance != current {
            return Err(AiError::StaleProvenance);
        }
        Ok(())
    }
}
