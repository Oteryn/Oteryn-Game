//! A real destination306 selects the source AddItem callback before the item loop commits.
use super::*;
use crate::durability::item_loop_move::SeedDropTarget;
use crate::durability::terrain_tool::TerrainScope;
use crate::map::boot::BundleMap;

pub(super) fn target(
    map: &BundleMap,
    to: WorldTilePosition,
    content: [u8; 32],
    epoch: u64,
) -> Option<SeedDropTarget> {
    let pos = TilePos {
        x: u16::try_from(to.x).ok()?,
        y: u16::try_from(to.y).ok()?,
        floor: i8::try_from(to.floor).ok()?,
    };
    let facts = map.facts.pinned();
    if facts.house_tile(pos) {
        return None;
    }
    let tile = view::compose_tile(&map.overlay, &facts, pos).ok()??;
    let selected = tile
        .entries
        .iter()
        .filter(|entry| entry.facts.appearance_id == 306)
        .collect::<Vec<_>>();
    let [entry] = selected.as_slice() else {
        return None;
    };
    if entry.facts.bound || entry.facts.count != 1 || entry.facts.sub_type != 0 {
        return None;
    }
    let crate::map::view::MapEntryKey::Base(key) = entry.source.key() else {
        return None;
    };
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Some(SeedDropTarget {
        scope: TerrainScope {
            map_revision: map_revision(map.overlay.base()),
            content_generation: content,
            reset_epoch: i64::try_from(epoch).ok()?,
        },
        placement,
        expected_revision: i64::try_from(facts.object_revision(key)).ok()?,
        ordinary_plant_admitted: [324, 330]
            .into_iter()
            .all(|id| map.facts.quest_callback_replacement(key, id).is_some()),
        ice_plant_admitted: [14031, 14032]
            .into_iter()
            .all(|id| map.facts.quest_callback_replacement(key, id).is_some()),
    })
}
