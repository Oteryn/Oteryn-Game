//! A real1x1 renderer grid holds the bounded original384-pixel CPU graph.
use oteryn_client_ui::wheel_allocation::{WheelPanel, diagram::graph};
use oteryn_renderer::{AtlasImage, TileBatch, TileCoord, TileView};
pub(super) fn frame(
    p: &WheelPanel,
) -> Result<Option<(AtlasImage, TileBatch)>, oteryn_renderer::BatchError> {
    let Some(rgba) = graph::rgba(p) else {
        return Ok(None);
    };
    raster_frame(rgba).map(Some)
}
fn raster_frame(rgba: Vec<u8>) -> Result<(AtlasImage, TileBatch), oteryn_renderer::BatchError> {
    let atlas = AtlasImage::new(384, 1, 1, rgba)?;
    let view = TileView::new(TileCoord::new(0, 0), 384, 1, 1)?;
    let batch = TileBatch::from_cells(&view, &atlas, &[0])?;
    Ok((atlas, batch))
}

#[cfg(windows)]
#[path = "wheel_surface_native.rs"]
mod native;
#[cfg(windows)]
pub(super) use native::Surface;
#[cfg(test)]
#[path = "tests/wheel_surface.rs"]
mod tests;
