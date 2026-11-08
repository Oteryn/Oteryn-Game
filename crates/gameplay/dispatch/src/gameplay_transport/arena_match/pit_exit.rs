//! Crystal ff7ede59 svargrond_arena.lua: pillar=1841, teleport=5022.
use super::*;
use crate::map::{
    boot::BundleMap,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};

pub(super) fn pillar(pit: u8) -> Option<TilePos> {
    let at = center(pit)?;
    Some(TilePos {
        x: (at.x - 6).try_into().ok()?,
        y: at.y.try_into().ok()?,
        floor: -7,
    })
}

pub(super) fn project(map: &BundleMap, pit: u8, open: bool) -> bool {
    let Some(pos) = pillar(pit) else { return false };
    let Some(base) = map.overlay.base().tile(pos.x, pos.y, pos.floor) else {
        return false;
    };
    let mut entries = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            (map.facts.quest_callback_base_appearance(pos, ordinal, *id) == Some(1841))
                .then_some((ordinal, *id))
        });
    let Some((ordinal, id)) = entries.next() else {
        return false;
    };
    if entries.next().is_some() {
        return false;
    }
    let Some(key) = placement_key(pos, ordinal) else {
        return false;
    };
    let appearance = if open { 5022 } else { 1841 };
    if map
        .facts
        .base_entry(pos, ordinal, id)
        .is_some_and(|facts| facts.appearance_id == appearance)
    {
        return true;
    }
    let Some(mut after) = map.facts.movement_item_facts(appearance) else {
        return false;
    };
    after.bound = true;
    after.pickupable = false;
    map.facts.apply_quest_callback(
        key,
        map.facts.object_revision(key),
        after,
        &[],
        &[(key, after, open)],
        false,
        None,
    )
}
