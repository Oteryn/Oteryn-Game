use super::{ChildOccurrenceRef, InteractionError, RootSourceOccurrenceRef};
use std::collections::BTreeMap;

const MAX_CASCADE_DEPTH: usize = 2;
const MAX_CHILD_FANOUT: usize = 8;
const MAX_ROOT_WORK: usize = 8;
const MAX_TRIGGER_CANDIDATES: usize = 16;
const MAX_RETAINED_CHILD_LIFECYCLES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerRegistration {
    key: String,
}

impl TriggerRegistration {
    pub fn new(key: &str) -> Result<Self, InteractionError> {
        if key.trim().is_empty() {
            return Err(InteractionError::EmptySemanticKey);
        }
        Ok(Self {
            key: key.to_owned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedChildWork {
    registration: TriggerRegistration,
    child: ChildOccurrenceRef,
}

impl SelectedChildWork {
    #[must_use]
    pub fn new(registration: TriggerRegistration, child: ChildOccurrenceRef) -> Self {
        Self {
            registration,
            child,
        }
    }

    fn registration(&self) -> &TriggerRegistration {
        &self.registration
    }

    fn child(&self) -> &ChildOccurrenceRef {
        &self.child
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedChildLifecycles {
    root: RootSourceOccurrenceRef,
    entries: Vec<ChildOccurrenceRef>,
}

impl RetainedChildLifecycles {
    #[must_use]
    pub fn new(root: RootSourceOccurrenceRef, entries: Vec<ChildOccurrenceRef>) -> Self {
        Self { root, entries }
    }

    fn root(&self) -> &RootSourceOccurrenceRef {
        &self.root
    }

    fn contains(&self, child: &ChildOccurrenceRef) -> bool {
        self.entries.iter().any(|entry| entry == child)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionPlan {
    root: RootSourceOccurrenceRef,
    children: Vec<ChildOccurrenceRef>,
}

impl InteractionPlan {
    pub fn build(
        root: RootSourceOccurrenceRef,
        mut registrations: Vec<TriggerRegistration>,
        mut selected_children: Vec<SelectedChildWork>,
        retained: RetainedChildLifecycles,
    ) -> Result<Self, InteractionError> {
        if retained.root() != &root {
            return Err(InteractionError::MismatchedRoot);
        }
        enforce_limit(
            "trigger candidates",
            MAX_TRIGGER_CANDIDATES,
            registrations.len(),
        )?;
        registrations.sort_by(|left, right| left.key.cmp(&right.key));
        if registrations.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(InteractionError::DuplicateTriggerRegistration);
        }
        selected_children.sort_by(|left, right| left.child.cmp(&right.child));

        let mut children = Vec::with_capacity(selected_children.len());
        let mut fanout_by_parent = BTreeMap::new();
        for selected in &selected_children {
            if !registrations.contains(selected.registration()) {
                return Err(InteractionError::UnregisteredSelectedChild);
            }
            let child = selected.child();
            if child.root() != &root {
                return Err(InteractionError::MismatchedRoot);
            }
            enforce_limit(
                "cascade depth",
                MAX_CASCADE_DEPTH,
                child.checked_ancestry_depth()?,
            )?;
            let fanout = fanout_by_parent.entry(child.parent()).or_insert(0_usize);
            *fanout = fanout
                .checked_add(1)
                .ok_or(InteractionError::CountOverflow)?;
            enforce_limit("child fan-out", MAX_CHILD_FANOUT, *fanout)?;
            children.push(child.clone());
        }
        if children.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(InteractionError::DuplicateChild);
        }
        enforce_limit("root work", MAX_ROOT_WORK, children.len())?;
        enforce_limit(
            "retained child lifecycles",
            MAX_RETAINED_CHILD_LIFECYCLES,
            retained.len(),
        )?;
        if children.iter().any(|child| !retained.contains(child)) {
            return Err(InteractionError::MissingRetainedLifecycle);
        }

        Ok(Self { root, children })
    }

    #[must_use]
    pub const fn root(&self) -> &RootSourceOccurrenceRef {
        &self.root
    }

    #[must_use]
    pub fn children(&self) -> &[ChildOccurrenceRef] {
        &self.children
    }
}

fn enforce_limit(
    resource: &'static str,
    limit: usize,
    observed: usize,
) -> Result<(), InteractionError> {
    if observed > limit {
        return Err(InteractionError::CapacityExceeded {
            resource,
            limit,
            observed,
        });
    }
    Ok(())
}
