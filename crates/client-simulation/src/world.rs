//! Deterministic stack elevation and source-cell reach for world projection.

use oteryn_client_domain::world::{CELL_PX, Draw, DrawCell, MapTile};

/// The client's cap on how far elevation lifts what is drawn above it.
pub const MAX_ELEVATION: i32 = 24;
/// Tiles scanned past the view on the right and bottom for cells reaching up or left.
pub const SCAN_MARGIN: i32 = 2;
const MAX_REACH_PX: i32 = SCAN_MARGIN * CELL_PX as i32;

/// How far each item of a sorted stack is lifted, and the tile's elevation. An item is lifted by
/// its own height (the appearance profile's displacement) on top of the heights below it; a top
/// item does not stand on the stack.
pub fn stack_lifts(items: impl IntoIterator<Item = (u8, i32)>) -> (Vec<i32>, i32) {
    let mut below = 0;
    let lifts = items
        .into_iter()
        .map(|(layer, height)| {
            if layer == 4 {
                return height;
            }
            let lift = (below + height).min(MAX_ELEVATION);
            below = lift;
            lift
        })
        .collect();
    (lifts, below)
}

/// An appearance's elevation as a stack height, capped like the stack itself so an oversized
/// profile value can neither wrap negative nor overflow the sum below it.
pub fn item_height(elevation: u32) -> i32 {
    i32::try_from(elevation).map_or(MAX_ELEVATION, |height| height.min(MAX_ELEVATION))
}

/// Whether a map cell at `offset` (lift included) stays within [`MAX_REACH_PX`] of its tile,
/// so the scene's scan past the view reaches it.
pub fn within_reach(offset: [i32; 2]) -> bool {
    offset.iter().all(|axis| *axis >= -MAX_REACH_PX)
}

/// Project one assigned source cell after applying its stack lift.
pub fn project_cell(
    cell: &DrawCell,
    lift: i32,
    slot: impl Fn(&DrawCell) -> Option<u16>,
) -> Option<Draw> {
    let slot = slot(cell)?;
    let offset = [cell.offset_x - lift, cell.offset_y - lift];
    (slot != 0 && within_reach(offset)).then_some(Draw { cell: slot, offset })
}

/// Project a sorted stack, preserving order within its under/over layers.
pub fn project_tile(
    items: &[(u8, u32, Vec<DrawCell>)],
    slot: impl Fn(&DrawCell) -> Option<u16>,
) -> MapTile {
    let mut tile = MapTile::default();
    let (lifts, elevation) =
        stack_lifts(items.iter().map(|(layer, e, _)| (*layer, item_height(*e))));
    for ((layer, _, item_cells), lift) in items.iter().zip(lifts) {
        let drawn = item_cells
            .iter()
            .filter_map(|cell| project_cell(cell, lift, &slot));
        if *layer == 4 {
            tile.over.extend(drawn);
        } else {
            tile.under.extend(drawn);
        }
    }
    tile.elevation = elevation;
    tile
}

#[cfg(test)]
mod tests;
