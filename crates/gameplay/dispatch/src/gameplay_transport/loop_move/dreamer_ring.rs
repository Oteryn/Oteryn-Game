//! Source UID2244 is selected from the actual composed destination of an item drop.
use super::*;
use crate::durability::item_loop_move::DreamerRingTarget;
use crate::durability::terrain_tool::TerrainScope;
use crate::map::boot::BundleMap;
use crate::map::view::EntryFacts;

pub(super) struct Prepared {
    pub(super) target: DreamerRingTarget,
    source: u64,
    revision: u64,
    facts: EntryFacts,
    trees: Vec<(u64, TilePos)>,
}

pub(super) fn target(
    map: &BundleMap,
    to: WorldTilePosition,
    content: [u8; 32],
    epoch: u64,
) -> Option<Prepared> {
    let pos = TilePos {
        x: to.x.try_into().ok()?,
        y: to.y.try_into().ok()?,
        floor: to.floor.try_into().ok()?,
    };
    let pinned = map.facts.pinned();
    if pinned.house_tile(pos) {
        return None;
    }
    let tile = view::compose_tile(&map.overlay, &pinned, pos).ok()??;
    let selected = tile
        .entries
        .iter()
        .filter_map(|entry| {
            let view::MapEntryKey::Base(key) = entry.source.key() else {
                return None;
            };
            map.facts
                .quest_callback_matches(key, Some(2244), None, None)
                .then_some((key, entry.facts))
        })
        .collect::<Vec<_>>();
    let [(source, facts)] = selected.as_slice() else {
        return None;
    };
    let revision = map.facts.object_revision(*source);
    let mut trees = Vec::new();
    let mut trees_before = [0; 3];
    for (index, position) in oteryn_gameplay_quests::quest::dreamer_death_ring::TREES
        .into_iter()
        .enumerate()
    {
        let pos = TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        if pinned.house_tile(pos) {
            return None;
        }
        let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
        let candidates = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .filter_map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key =
                    oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, ordinal)?;
                let facts = map.facts.base_entry(pos, ordinal, *id)?;
                (facts.appearance_id == 3636).then_some(key)
            })
            .collect::<Vec<_>>();
        let [key] = candidates.as_slice() else {
            return None;
        };
        if !pinned.base_hidden(*key) {
            trees_before[index] = 3636;
            trees.push((*key, pos));
        }
    }
    if trees.is_empty()
        || !map
            .facts
            .quest_callback_capacity(*source, revision, trees.len())
    {
        return None;
    }
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&source.to_be_bytes());
    Some(Prepared {
        source: *source,
        revision,
        facts: *facts,
        trees,
        target: DreamerRingTarget {
            scope: TerrainScope {
                map_revision: map_revision(map.overlay.base()),
                content_generation: content,
                reset_epoch: epoch.try_into().ok()?,
            },
            placement,
            expected_revision: revision.try_into().ok()?,
            trees_before,
        },
    })
}

pub(super) fn publish(map: &BundleMap, prepared: &Prepared) -> bool {
    let keys = prepared
        .trees
        .iter()
        .map(|(key, _)| *key)
        .collect::<Vec<_>>();
    if !map.facts.apply_quest_callback(
        prepared.source,
        prepared.revision,
        prepared.facts,
        &keys,
        &[],
        false,
        None,
    ) {
        return false;
    }
    for (_, position) in &prepared.trees {
        map.facts.emit_movement_effect(*position, 13);
    }
    true
}
