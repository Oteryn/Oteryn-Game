//! Pure qualified-content checks for attack and chase target cells.

use oteryn_gameplay_content::content::{
    CollisionClass, LogicalCell, NativeEntryMovementCells, QualifiedNativeEntryRoom,
    SpellTileLookupError, native_cell_lookup,
};

pub fn in_protection_zone(room: &QualifiedNativeEntryRoom, position: LogicalCell) -> Option<bool> {
    let cells = room.movement_cells();
    cells
        .spell_tiles()
        .lookup(cells.scope(), position)
        .ok()
        .map(|tile| tile.flags().protection_zone)
}

/// CHASE-1: a Walkable cell with no protection zone or floor change. Unknown tile
/// facts refuse, except on the accepted entry room without a tile document, whose Movement owner
/// admits a step on terrain and doors alone. Dispatch separately checks closed doors and teleports.
pub fn chase_path_tile(
    cells: &NativeEntryMovementCells,
    target: LogicalCell,
    source_profile: bool,
) -> bool {
    let ordinary = match cells.spell_tiles().lookup(cells.scope(), target) {
        Ok(tile) => !tile.flags().protection_zone && !tile.flags().floor_change,
        Err(SpellTileLookupError::Unknown) => {
            matches!(
                cells.index(),
                native_cell_lookup::NativeMovementCollisionIndex::Entry(_)
            ) && !source_profile
        }
        Err(_) => false,
    };
    ordinary && cells.index().lookup(cells.scope(), target) == Ok(CollisionClass::Walkable)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use oteryn_gameplay_content::content::native_gameplay_test_world_id;
    use oteryn_gameplay_content::content::qualify_native_entry_room;

    #[test]
    fn entry_chase_uses_known_tile_flags_and_requires_known_walkable_terrain() {
        let room = qualify_native_entry_room(native_gameplay_test_world_id().unwrap()).unwrap();
        let start = room.entry_start();
        let cell = LogicalCell {
            x: start.x,
            y: start.y,
            z: i32::from(start.floor),
        };
        let cells = room.movement_cells();
        assert!(chase_path_tile(cells, cell, false));
        assert!(cells.spell_tiles().lookup(cells.scope(), cell).is_ok());
        assert!(chase_path_tile(cells, cell, true));
        assert_eq!(in_protection_zone(&room, cell), Some(false));
        let unknown = LogicalCell {
            x: i32::MAX,
            ..cell
        };
        assert!(!chase_path_tile(cells, unknown, false));
        assert!(!chase_path_tile(cells, unknown, true));
        assert_eq!(in_protection_zone(&room, unknown), None);
    }
}
