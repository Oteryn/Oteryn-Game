use super::InteractionError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SemanticKey(String);

impl SemanticKey {
    fn new(value: &str) -> Result<Self, InteractionError> {
        if value.trim().is_empty() {
            return Err(InteractionError::EmptySemanticKey);
        }
        Ok(Self(value.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootSourceOccurrenceRef(SemanticKey);

impl RootSourceOccurrenceRef {
    pub fn new(value: &str) -> Result<Self, InteractionError> {
        SemanticKey::new(value).map(Self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SemanticRevisionContext {
    content: SemanticKey,
    ruleset: SemanticKey,
    simulation: SemanticKey,
}

impl SemanticRevisionContext {
    pub fn new(content: &str, ruleset: &str, simulation: &str) -> Result<Self, InteractionError> {
        Ok(Self {
            content: SemanticKey::new(content)?,
            ruleset: SemanticKey::new(ruleset)?,
            simulation: SemanticKey::new(simulation)?,
        })
    }
}

/// Current owner evidence required to apply or complete delegated work.
///
/// This is deliberately separate from child occurrence identity: failover or
/// a mutable revision change must not create a fresh logical child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityFenceEvidence {
    ownership_generation: u64,
    state_revision: u64,
    domain_revision: u64,
}

impl AuthorityFenceEvidence {
    pub fn new(
        ownership_generation: u64,
        state_revision: u64,
        domain_revision: u64,
    ) -> Result<Self, InteractionError> {
        if ownership_generation == 0 || state_revision == 0 || domain_revision == 0 {
            return Err(InteractionError::InvalidAuthorityFence);
        }
        Ok(Self {
            ownership_generation,
            state_revision,
            domain_revision,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ParentOccurrenceRef {
    Root(RootSourceOccurrenceRef),
    Child(Box<ChildOccurrenceRef>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChildOccurrenceRef {
    parent: ParentOccurrenceRef,
    definition: SemanticKey,
    target: SemanticKey,
    edge: SemanticKey,
    ordinal: Option<u16>,
    revisions: SemanticRevisionContext,
}

impl ChildOccurrenceRef {
    pub fn for_root(
        root: &RootSourceOccurrenceRef,
        definition: &str,
        target: &str,
        edge: &str,
        ordinal: Option<u16>,
        revisions: &SemanticRevisionContext,
    ) -> Result<Self, InteractionError> {
        Self::new(
            ParentOccurrenceRef::Root(root.clone()),
            definition,
            target,
            edge,
            ordinal,
            revisions,
        )
    }

    pub fn for_child(
        parent: &Self,
        definition: &str,
        target: &str,
        edge: &str,
        ordinal: Option<u16>,
        revisions: &SemanticRevisionContext,
    ) -> Result<Self, InteractionError> {
        Self::new(
            ParentOccurrenceRef::Child(Box::new(parent.clone())),
            definition,
            target,
            edge,
            ordinal,
            revisions,
        )
    }

    fn new(
        parent: ParentOccurrenceRef,
        definition: &str,
        target: &str,
        edge: &str,
        ordinal: Option<u16>,
        revisions: &SemanticRevisionContext,
    ) -> Result<Self, InteractionError> {
        Ok(Self {
            parent,
            definition: SemanticKey::new(definition)?,
            target: SemanticKey::new(target)?,
            edge: SemanticKey::new(edge)?,
            ordinal,
            revisions: revisions.clone(),
        })
    }

    #[must_use]
    pub fn ancestry_depth(&self) -> usize {
        match self.checked_ancestry_depth() {
            Ok(depth) => depth,
            Err(InteractionError::CountOverflow) => usize::MAX,
            Err(_) => 0,
        }
    }

    pub(crate) fn root(&self) -> &RootSourceOccurrenceRef {
        let mut parent = &self.parent;
        loop {
            match parent {
                ParentOccurrenceRef::Root(root) => return root,
                ParentOccurrenceRef::Child(child) => parent = &child.parent,
            }
        }
    }

    pub(crate) fn parent(&self) -> &ParentOccurrenceRef {
        &self.parent
    }

    pub(crate) fn checked_ancestry_depth(&self) -> Result<usize, InteractionError> {
        let mut depth = 1_usize;
        let mut parent = &self.parent;
        while let ParentOccurrenceRef::Child(child) = parent {
            depth = depth
                .checked_add(1)
                .ok_or(InteractionError::CountOverflow)?;
            parent = &child.parent;
        }
        Ok(depth)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RngDecisionRef {
    child: ChildOccurrenceRef,
    purpose: SemanticKey,
    draw_ordinal: u16,
}

impl RngDecisionRef {
    pub fn new(
        child: &ChildOccurrenceRef,
        purpose: &str,
        draw_ordinal: u16,
    ) -> Result<Self, InteractionError> {
        Ok(Self {
            child: child.clone(),
            purpose: SemanticKey::new(purpose)?,
            draw_ordinal,
        })
    }
}
