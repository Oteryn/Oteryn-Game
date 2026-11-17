//! Enumerate every source loot entry that still lacks admissible Item facts.
use crate::content::creature_reward::{SectionItemFacts, SectionLootEntry};

pub(super) fn items<'a>(
    entries: &'a [SectionLootEntry],
    facts: &[SectionItemFacts],
) -> Vec<&'a str> {
    let mut refused = Vec::new();
    for entry in entries {
        if facts
            .iter()
            .find(|fact| fact.item == entry.item)
            .is_none_or(|fact| {
                !fact.materializable
                    || (!entry.children.is_empty() && fact.container_capacity.is_none())
            })
        {
            refused.push(entry.item.key.as_str());
        }
        refused.extend(items(&entry.children, facts));
    }
    refused
}
