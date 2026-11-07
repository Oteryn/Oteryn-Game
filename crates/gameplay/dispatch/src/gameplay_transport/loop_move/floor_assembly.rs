//! Resolve the actual bound AddItem altar through the current composed map.
use super::*;
use crate::durability::item_loop_move::FloorAssemblyTarget;
use crate::durability::terrain_tool::TerrainScope;
use crate::map::boot::BundleMap;

pub(super) fn target(
    map: &BundleMap,
    to: WorldTilePosition,
    content: [u8; 32],
    epoch: u64,
) -> Option<FloorAssemblyTarget> {
    let pos = TilePos {
        x: to.x.try_into().ok()?,
        y: to.y.try_into().ok()?,
        floor: to.floor.try_into().ok()?,
    };
    let facts = map.facts.pinned();
    if facts.house_tile(pos) {
        return None;
    }
    let tile = view::compose_tile(&map.overlay, &facts, pos).ok()??;
    let mut selected = None;
    for entry in &tile.entries {
        let crate::map::view::MapEntryKey::Base(key) = entry.source.key() else {
            continue;
        };
        for action in oteryn_gameplay_items::floor_assembly::actions() {
            if map
                .facts
                .quest_callback_matches(key, None, Some(action), None)
            {
                if selected.is_some() {
                    return None;
                }
                selected = Some((key, action));
            }
        }
    }
    let (key, action) = selected?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Some(FloorAssemblyTarget {
        action,
        placement,
        scope: TerrainScope {
            map_revision: map_revision(map.overlay.base()),
            content_generation: content,
            reset_epoch: epoch.try_into().ok()?,
        },
    })
}
