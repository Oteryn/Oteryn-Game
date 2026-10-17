//! Pure, bounded Imbuement checkpoint planning over supplied row DATA.
//! This preserves bindings and monotonic remaining values; it does not prove elapsed time.

use crate::domain::ItemInstanceId;
use crate::domain::imbuement::{MAX_SLOTS, SlotRecord};
use std::collections::{BTreeMap, BTreeSet};

/// IMBFORGE0-RL-13 and DUR03-RL-06-IMBUE-CHECKPOINT-{PARTICIPANTS,EFFECT-WORK-UNITS}.
pub(crate) const MAX_CHECKPOINT_ITEMS: usize = 10;
pub(crate) const MAX_CHECKPOINT_WORK: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckpointError {
    BindingChanged,
    TimeIncreased,
    DuplicateSlot,
    DuplicateCategory,
    BudgetExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlotCheckpoint {
    pub(crate) before: SlotRecord,
    pub(crate) after: SlotRecord,
}

/// Numeric ceilings only; passing them is not audit-shape/current-authority validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CheckpointUsage {
    pub(crate) items: usize,
    pub(crate) participants: usize,
    pub(crate) work: usize,
}

impl CheckpointUsage {
    pub(crate) fn check(self) -> Result<(), CheckpointError> {
        if self.items > MAX_CHECKPOINT_ITEMS
            || self.participants > MAX_CHECKPOINT_ITEMS
            || self.work > MAX_CHECKPOINT_WORK
        {
            Err(CheckpointError::BudgetExceeded)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckpointItem {
    pub(crate) item: ItemInstanceId,
    pub(crate) changes: Vec<SlotCheckpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckpointBatch {
    pub(crate) items: Vec<CheckpointItem>,
    pub(crate) usage: CheckpointUsage,
}

impl CheckpointBatch {
    /// Groups/sorts one bounded transaction's input; it does not split or commit a character's
    /// entire interval. Equal snapshots are omitted; zero remaining is an update, never deletion.
    pub(crate) fn plan(changes: Vec<SlotCheckpoint>) -> Result<Self, CheckpointError> {
        if changes.len() > MAX_CHECKPOINT_ITEMS * MAX_SLOTS {
            return Err(CheckpointError::BudgetExceeded);
        }
        let mut grouped: BTreeMap<ItemInstanceId, Vec<SlotCheckpoint>> = BTreeMap::new();
        let mut slots = BTreeSet::new();
        let mut categories = BTreeSet::new();
        for change in changes {
            let binding = change.before.binding();
            if binding != change.after.binding() {
                return Err(CheckpointError::BindingChanged);
            }
            if change.after.remaining_ms() > change.before.remaining_ms() {
                return Err(CheckpointError::TimeIncreased);
            }
            if !slots.insert((binding.item, binding.slot)) {
                return Err(CheckpointError::DuplicateSlot);
            }
            if !categories.insert((binding.item, binding.category.clone())) {
                return Err(CheckpointError::DuplicateCategory);
            }
            grouped.entry(binding.item).or_default().push(change);
        }
        if grouped.len() > MAX_CHECKPOINT_ITEMS {
            return Err(CheckpointError::BudgetExceeded);
        }
        let mut items = Vec::new();
        let mut work = 0;
        for (item, mut changes) in grouped {
            changes.retain(|change| change.before != change.after);
            if changes.is_empty() {
                continue;
            }
            changes.sort_by_key(|change| change.before.binding().slot);
            work += 1 + changes.len();
            items.push(CheckpointItem { item, changes });
        }
        let usage = CheckpointUsage {
            items: items.len(),
            participants: items.len(),
            work,
        };
        usage.check()?;
        Ok(Self { items, usage })
    }
}
