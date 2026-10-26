//! Deterministic stack elevation and source-cell reach for world projection.

use oteryn_client_domain::world::CELL_PX;

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

#[cfg(test)]
mod tests;
