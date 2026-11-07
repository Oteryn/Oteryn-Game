//! Resolve the whole placement predicate from one current callback/door snapshot.
use super::*;
use crate::map::view::EntrySource;
use oteryn_gameplay_quests::quest::world_callbacks::ObjectCondition;

pub(super) fn satisfied(map: &BundleMap, conditions: &[ObjectCondition]) -> bool {
    if conditions.is_empty() {
        return true;
    }
    let facts = map.facts.pinned();
    conditions.iter().all(|condition| {
        let position = tile(condition.position);
        !facts.house_tile(position)
            && crate::map::view::compose_tile(&map.overlay, &facts, position)
                .ok()
                .flatten()
                .is_some_and(|current| {
                    current.entries.iter().any(|entry| {
                        entry.facts.appearance_id == condition.appearance
                            && !entry.facts.pickupable
                            && matches!(
                                entry.source,
                                EntrySource::Base { movable: false, .. } | EntrySource::Added(_)
                            )
                    })
                })
    })
}
