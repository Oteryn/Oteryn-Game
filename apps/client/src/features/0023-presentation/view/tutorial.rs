use super::{AtlasImage, BatchError, PresentationColor, SpriteBatch, TileView, pixel};
use std::time::Instant;

pub(super) fn draw_tutorial(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    overlay: &crate::play::tutorial_overlay::Overlay,
    now: Instant,
) -> Result<(), BatchError> {
    if !overlay.enabled {
        return Ok(());
    }
    for [x, y] in overlay.points(now) {
        pixel(
            batch,
            view,
            atlas,
            [x as f32, y as f32],
            [3.0, 3.0],
            PresentationColor::Yellow,
        )?;
    }
    if let Some([x, y]) = overlay.marker {
        for (dx, dy) in [(-8, 0), (8, 0), (0, -8), (0, 8)] {
            pixel(
                batch,
                view,
                atlas,
                [(x + dx) as f32, (y + dy) as f32],
                [4.0, 4.0],
                PresentationColor::LightGreen,
            )?;
        }
    }
    for (index, character) in overlay.hint().chars().enumerate() {
        let glyph = oteryn_renderer::name_text::glyph_rgba(character);
        for y in 0..7usize {
            for x in 0..5usize {
                if glyph[(((y * 32 + 16) / 8) * 32 + (x * 32 + 16) / 6) * 4 + 3] == 255 {
                    pixel(
                        batch,
                        view,
                        atlas,
                        [8.0 + index as f32 * 6.0 + x as f32, 8.0 + y as f32],
                        [1.0, 1.0],
                        PresentationColor::White,
                    )?;
                }
            }
        }
    }
    Ok(())
}
