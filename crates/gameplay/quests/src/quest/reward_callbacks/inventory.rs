//! Reuse L2 1819e54c7: zero is a real absence predicate, not a positive hand-in.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InventoryRequirement {
    pub item: String,
    pub minimum: u32,
    pub maximum: u32,
}

impl InventoryRequirement {
    pub fn holds(&self, quantities: &BTreeMap<String, u64>) -> bool {
        let quantity = quantities.get(&self.item).copied().unwrap_or(0);
        (u64::from(self.minimum)..=u64::from(self.maximum)).contains(&quantity)
    }
}
