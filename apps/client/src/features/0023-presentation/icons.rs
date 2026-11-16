//! Tibia's flag artwork for the observer-relative party and skull wire values; original compact
//! glyphs when the graphics bundle is absent.
use super::{HudIconCells, party_icon_rgb, pixel_rgb};
use oteryn_renderer::{AtlasImage, BatchError, QuadInstance, SpriteBatch, TileView};
use std::time::Duration;

const OUTLINE: [u8; 8] = [0x7e, 0x42, 0x42, 0x42, 0x42, 0x24, 0x18, 0];
const MEMBER: [u8; 8] = [0x7e, 0x7e, 0x7e, 0x7e, 0x7e, 0x3c, 0x18, 0];
const SHARED: [u8; 8] = [0x7e, 0x42, 0x5a, 0x7e, 0x5a, 0x24, 0x18, 0];
const INACTIVE: [u8; 8] = [0x7e, 0x46, 0x4a, 0x52, 0x62, 0x24, 0x18, 0];
const SKULL: [u8; 8] = [0x3c, 0x7e, 0xdb, 0xdb, 0xff, 0x66, 0x3c, 0x24];

// The artwork route adds the cells and scale to the wire values the glyph route already takes.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    cells: Option<&HudIconCells>,
    at: [f32; 2],
    value: u8,
    skull: bool,
    age: Duration,
    scale: f32,
) -> Result<(), BatchError> {
    if value == 0 || (!skull && matches!(value, 7 | 8) && age.as_millis() / 500 % 2 == 1) {
        return Ok(());
    }
    let index = usize::from(value) - 1;
    let cell = cells.and_then(|cells| {
        if skull {
            cells.skulls.get(index)
        } else {
            cells.shields.get(index)
        }
    });
    // The icon fills the top left of its cell; an atlas built before the bundle lacks it.
    if let Some(&cell) = cell.filter(|cell| u32::from(**cell) < atlas.cell_count()) {
        if batch.len() < oteryn_renderer::MAX_BATCH_QUADS {
            batch.push_quad(QuadInstance {
                position: at,
                size: [32.0 * scale; 2],
                uv: atlas.uv_rect(cell)?,
            })?;
        }
        return Ok(());
    }
    let pattern = if skull {
        SKULL
    } else {
        match value {
            3 | 4 => MEMBER,
            5 | 6 => SHARED,
            7..=10 => INACTIVE,
            _ => OUTLINE,
        }
    };
    let color = party_icon_rgb(value, skull);
    for (row, bits) in pattern.into_iter().enumerate() {
        for column in 0..8 {
            if bits & (128 >> column) != 0 {
                let point = [at[0] + column as f32 * 1.5, at[1] + row as f32 * 1.5];
                // Black skulls retain an emissive outline on dark terrain.
                if skull && value == 5 {
                    pixel_rgb(
                        batch,
                        view,
                        atlas,
                        [point[0] - 0.5, point[1] - 0.5],
                        [2.5, 2.5],
                        [5; 3],
                    )?;
                }
                pixel_rgb(batch, view, atlas, point, [1.5, 1.5], color)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_party_states_keep_distinct_membership_and_sharing_shapes() {
        assert_ne!(OUTLINE, MEMBER);
        assert_ne!(MEMBER, SHARED);
        assert_ne!(SHARED, INACTIVE);
        assert_ne!(SKULL, OUTLINE);
    }
}
