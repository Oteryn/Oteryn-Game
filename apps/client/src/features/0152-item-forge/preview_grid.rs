//! The renderer accepts tile columns/rows here, not viewport pixels.
use oteryn_renderer::{BatchError, TileCoord, TileView};
pub(super) fn item_preview_view() -> Result<TileView, BatchError> {
    TileView::new(TileCoord::new(0, 0), 32, 4, 4)
}
#[cfg(test)]
#[path = "preview_grid_tests.rs"]
mod tests;
