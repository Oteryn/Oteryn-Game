//! Read all source cells under the Channel owner; publish the selected branch atomically.
use super::*;
use crate::map::facts::QuestCallbackProjection;
use crate::map::view::EntryFacts;
use oteryn_gameplay_quests::quest::children_puzzle as puzzle;

type Cells = [(u64, u64, EntryFacts); 4];
fn read(map: &BundleMap) -> Option<Cells> {
    let mut cells = Vec::new();
    for (position, pair) in puzzle::CELLS.iter().zip(puzzle::PAIRS) {
        let position = tile(*position);
        if map.facts.house_tile(position) {
            return None;
        }
        let base = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)?;
        let mut candidates = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .filter_map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key = placement_key(position, ordinal)?;
                let facts = map.facts.base_entry(position, ordinal, *id)?;
                (!map.facts.base_hidden(key)
                    && !facts.pickupable
                    && pair.contains(&facts.appearance_id))
                .then_some((key, map.facts.object_revision(key), facts))
            });
        let cell = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        cells.push(cell);
    }
    cells.try_into().ok()
}

pub(super) fn matches(map: &BundleMap, callback: &Callback) -> bool {
    let Some((index, solved)) = puzzle::binding(&callback.key) else {
        return true;
    };
    read(map)
        .and_then(|cells| puzzle::toggle(index, cells.map(|(_, _, facts)| facts.appearance_id)))
        .is_some_and(|after| (after == puzzle::SOLVED) == solved)
}

pub(super) fn prepare(
    map: &BundleMap,
    callback: &Callback,
    projection: Option<QuestCallbackProjection>,
) -> Result<Option<QuestCallbackProjection>, UseDisposition> {
    let Some((index, solved)) = puzzle::binding(&callback.key) else {
        return Ok(projection);
    };
    let cells = read(map).ok_or(UseDisposition::Rejected)?;
    let after = puzzle::toggle(index, cells.map(|(_, _, facts)| facts.appearance_id))
        .ok_or(UseDisposition::Rejected)?;
    if (after == puzzle::SOLVED) != solved {
        return Err(UseDisposition::StaleState);
    }
    let reset = match projection {
        Some(QuestCallbackProjection::TimedTransform { due, restore }) => Some((due, restore)),
        None => None,
        _ => return Err(UseDisposition::Rejected),
    };
    let mut entries = Vec::new();
    for ((key, revision, before), appearance) in cells.into_iter().zip(after) {
        let replacement = if before.appearance_id == appearance {
            None
        } else {
            Some(
                map.facts
                    .quest_callback_replacement(key, appearance)
                    .filter(|facts| !facts.pickupable)
                    .ok_or(UseDisposition::Rejected)?,
            )
        };
        entries.push((key, revision, replacement));
    }
    Ok(Some(QuestCallbackProjection::ReadsetTransforms {
        entries,
        reset,
    }))
}
