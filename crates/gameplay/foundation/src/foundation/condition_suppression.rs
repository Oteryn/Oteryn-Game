//! EQUIP-0 §4: ephemeral equipment suppression in the existing condition owner.

use super::ConditionStore;
use oteryn_gameplay_domain::domain::condition::ConditionType;
use std::collections::BTreeSet;

impl<S: Clone> ConditionStore<S> {
    /// EQUIP-0 §4: replace the active set and remove matching instances through the existing
    /// dispel path. The returned removal count has the same meaning as `dispel_key`.
    pub fn set_suppressed(&mut self, suppressed: BTreeSet<ConditionType>) -> usize {
        let removed = suppressed
            .iter()
            .map(|kind| self.dispel_key(kind.conflict_key()))
            .sum();
        self.suppressed = suppressed;
        removed
    }
}
