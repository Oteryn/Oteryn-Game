use super::item_preview_view;
use oteryn_renderer::{AtlasImage, BatchError, TileBatch, world::BLACK_CELL};
#[test]
fn forge_item_preview_grid_accepts_real_sixteen_cell_batch_at_128_pixels() -> Result<(), BatchError>
{
    let view = item_preview_view()?;
    let atlas = AtlasImage::new(32, 1, 1, vec![0; 32 * 32 * 4])?;
    let _batch = TileBatch::from_cells(&view, &atlas, &[BLACK_CELL; 16])?;
    assert_eq!(view.viewport_px(), (128, 128));
    Ok(())
}
