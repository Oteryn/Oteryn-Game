//! Transient combat overlays use server positions, including the own actor and hidden attackers.
//! Item effects retain their donor colours; fireworks expand into eight emissive sparks.
//! Other magic effects retain the existing hit pulse until official sprites are resolved.
use super::state::{MAX_VISUAL_CUES, VISUAL_LIFETIME, VisualCue};
use crate::play::PlayView;
use oteryn_renderer::{AtlasImage, BatchError, QuadInstance, SpriteBatch, TileCoord, TileView};
use oteryn_session::{PresentationColor, PresentationEventKind, PresentationValue};
use std::time::Instant;

include!("view/methods.rs");

// Original compact 3 × 5 decimal glyphs, most significant row first.
const DIGITS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];

fn number(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    [center, top]: [f32; 2],
    value: PresentationValue,
) -> Result<(), BatchError> {
    let text = value.value.to_string();
    let left = center - (text.len() as f32 * 8.0 - 2.0) / 2.0;
    for (i, digit) in text.bytes().enumerate() {
        for (row, bits) in DIGITS[usize::from(digit - b'0')].iter().enumerate() {
            for column in 0..3 {
                if bits & (4 >> column) != 0 {
                    pixel(
                        batch,
                        view,
                        atlas,
                        [left + (i * 8 + column * 2) as f32, top + row as f32 * 2.0],
                        [2.0, 2.0],
                        value.color,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn rgb(color: PresentationColor) -> [u8; 3] {
    use PresentationColor::*;
    match color {
        Red => [5, 0, 0],
        LightGreen => [2, 5, 2],
        LightGrey => [4, 4, 4],
        Orange => [5, 3, 0],
        Purple => [4, 1, 5],
        SkyBlue => [0, 4, 5],
        Yellow => [5, 5, 0],
        DarkRed => [3, 0, 0],
        LightBlue => [2, 3, 5],
        Blue => [0, 0, 5],
        PastelRed => [5, 2, 2],
        MayaBlue => [2, 4, 5],
        White => [5, 5, 5],
    }
}

fn pixel(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    position: [f32; 2],
    size: [f32; 2],
    color: PresentationColor,
) -> Result<(), BatchError> {
    pixel_rgb(batch, view, atlas, position, size, rgb(color))
}

fn pixel_rgb(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    position: [f32; 2],
    size: [f32; 2],
    [r, g, b]: [u8; 3],
) -> Result<(), BatchError> {
    let (width, height) = view.viewport_px();
    if position[0] < 0.0
        || position[1] < 0.0
        || position[0] + size[0] > width as f32
        || position[1] + size[1] > height as f32
    {
        return Ok(());
    }
    let index = u32::from(r) * 36 + u32::from(g) * 6 + u32::from(b);
    let cell = u32::from(oteryn_renderer::world::PALETTE_CELL);
    let columns = atlas.width() / atlas.cell_px();
    let x = (cell % columns) * atlas.cell_px() + index % atlas.cell_px();
    let y = (cell / columns) * atlas.cell_px() + index / atlas.cell_px();
    let u = (x as f32 + 0.5) / atlas.width() as f32;
    let v = (y as f32 + 0.5) / atlas.height() as f32;
    if batch.len() == oteryn_renderer::MAX_BATCH_QUADS {
        return Ok(());
    }
    batch.push_quad(QuadInstance {
        position,
        size,
        uv: [u, v, u, v],
    })
}

fn draw_tutorial(
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

// Observer-relative values follow party_skull::PartyIcon's wire contract.
fn party_icon_rgb(value: u8, skull: bool) -> [u8; 3] {
    if skull {
        match value {
            1 => [5, 5, 0],
            2 => [2, 5, 2],
            3 => [5, 5, 5],
            4 => [5, 0, 0],
            5 => [0, 0, 0],
            6 => [5, 3, 0],
            _ => [4, 4, 4],
        }
    } else {
        match value {
            1 | 3 | 5 | 7 | 9 => [0, 0, 5],
            2 | 4 | 6 | 8 | 10 => [5, 5, 0],
            _ => [4, 4, 4],
        }
    }
}

#[cfg(test)]
mod icon_palette_tests {
    use super::*;
    #[test]
    fn o021_skull_palette_preserves_server_meanings() {
        assert_eq!(party_icon_rgb(1, true), [5, 5, 0]);
        assert_eq!(party_icon_rgb(2, true), [2, 5, 2]);
        assert_eq!(party_icon_rgb(3, true), [5, 5, 5]);
        assert_eq!(party_icon_rgb(4, true), [5, 0, 0]);
        assert_eq!(party_icon_rgb(5, true), [0, 0, 0]);
        assert_eq!(party_icon_rgb(6, true), [5, 3, 0]);
        assert_eq!(party_icon_rgb(11, false), [4, 4, 4]);
    }
}
