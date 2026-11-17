//! Real renderer grid over original CPU glyphs. No external assets or sprite IDs.
use oteryn_client_ui::wheel_allocation::{WheelPanel, gems::atlas};
use oteryn_renderer::{AtlasImage, TileBatch, TileCoord, TileView};
pub(super) fn frame(
    p: &WheelPanel,
) -> Result<(AtlasImage, TileBatch), oteryn_renderer::BatchError> {
    let image = AtlasImage::new(64, 5, 1, atlas::panel_glyphs(p))?;
    let view = TileView::new(TileCoord::new(0, 0), 64, 5, 1)?;
    let batch = TileBatch::from_cells(&view, &image, &[0, 1, 2, 3, 4])?;
    Ok((image, batch))
}
#[cfg(test)]
mod tests {
    #[test]
    fn gem_atelier_real_tile_batch_has_five_glyph_cells() -> Result<(), oteryn_renderer::BatchError>
    {
        let (_, batch) = super::frame(&Default::default())?;
        assert_eq!(batch.len(), 5);
        Ok(())
    }
}
