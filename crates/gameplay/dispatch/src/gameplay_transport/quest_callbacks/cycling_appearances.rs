//! Crystal items.xml:24509-24548 and actions_mission02.lua:84-104.
//! The Channel rotates map appearances even when no player uses the cannon.
use super::*;
use crate::map::view::EntryFacts;
use std::collections::BTreeMap;
use world_callbacks::CyclingAppearanceGuard;

fn cell(map: &BundleMap, guard: CyclingAppearanceGuard) -> Option<(u64, EntryFacts, u16)> {
    let pos = tile(guard.position);
    let key = placement_key(pos, guard.ordinal)?;
    if map.facts.house_tile(pos) || map.facts.base_hidden(key) {
        return None;
    }
    let base = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
    let (id, _) = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(guard.ordinal))?;
    let current = map.facts.base_entry(pos, guard.ordinal, *id)?;
    let initial = map
        .facts
        .quest_callback_base_appearance(pos, guard.ordinal, *id)?;
    (!current.pickupable
        && guard.appearances.contains(&initial)
        && guard.appearances.contains(&current.appearance_id))
    .then_some((key, current, initial))
}

pub(super) fn matches(map: &BundleMap, callback: &Callback) -> bool {
    callback.effects.cycling_appearance.is_none_or(|guard| {
        cell(map, guard).is_some_and(|(_, current, _)| current.appearance_id == guard.required)
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called by the source owner's Channel turn under its current runtime fence.
    pub(in crate::gameplay_transport) fn advance_quest_appearances(
        &self,
        map: &BundleMap,
        now: u64,
    ) {
        let mut cycles: BTreeMap<_, Option<CyclingAppearanceGuard>> = BTreeMap::new();
        for guard in self
            .callback_content()
            .iter()
            .filter_map(|c| c.effects.cycling_appearance)
        {
            cycles
                .entry((guard.position, guard.ordinal))
                .and_modify(|prior| {
                    if prior.is_some_and(|old| !old.same_cycle(&guard)) {
                        *prior = None;
                    }
                })
                .or_insert(Some(guard));
        }
        for guard in cycles.into_values().flatten() {
            let Some((key, current, initial)) = cell(map, guard) else {
                continue;
            };
            let interval = u64::from(guard.interval_ms) * 1_000;
            if interval == 0 {
                continue;
            }
            let index = usize::from(initial == guard.appearances[1]);
            let target = guard.appearances[index ^ usize::from((now / interval) % 2 == 1)];
            if current.appearance_id != target
                && let Some(after) = map
                    .facts
                    .quest_callback_replacement(key, target)
                    .filter(|replacement| !replacement.pickupable)
            {
                let revision = map.facts.object_revision(key);
                map.facts
                    .apply_quest_callback(key, revision, after, &[], &[], false, None);
            }
        }
    }
}
