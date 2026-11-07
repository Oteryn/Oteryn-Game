//! Preserve prior native callback hashes without inventing donor NPC history.
use super::{NpcDefinitionPredecessor, QuestStateCatalogue};
use std::collections::BTreeSet;

pub(super) fn preserve_native_only(
    catalogue: &mut QuestStateCatalogue,
    source_owners: BTreeSet<String>,
) {
    let native_only: Vec<_> = catalogue
        .npc_legacy
        .keys()
        .filter(|quest| !source_owners.contains(*quest))
        .cloned()
        .collect();
    for quest in native_only {
        if let Some(versions) = catalogue.npc_legacy.remove(&quest) {
            let known = catalogue.predecessors.entry(quest).or_default();
            for (hash, defaults) in versions {
                if !known.iter().any(|entry| entry.hash == hash) {
                    known.push(NpcDefinitionPredecessor { hash, defaults });
                }
            }
        }
    }
}
