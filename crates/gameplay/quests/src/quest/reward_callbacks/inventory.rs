//! Source-authored inventory predicates; the fenced writer checks them again.
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absence_and_exact_quantity_do_not_accept_neighbors_or_overflow() {
        let item = "oteryn:item.tibia.i131".to_owned();
        let absent = InventoryRequirement {
            item: item.clone(),
            minimum: 0,
            maximum: 0,
        };
        let exact = InventoryRequirement {
            item: item.clone(),
            minimum: 2,
            maximum: 2,
        };
        assert!(absent.holds(&BTreeMap::new()));
        for quantity in [0, 1, 2, 3, u64::from(u32::MAX) + 1] {
            let observed = BTreeMap::from([(item.clone(), quantity)]);
            assert_eq!(absent.holds(&observed), quantity == 0);
            assert_eq!(exact.holds(&observed), quantity == 2);
        }
    }
}
