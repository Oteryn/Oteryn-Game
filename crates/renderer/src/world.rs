//! Static map atlas layout and builtin cells, independent of asset decoding.

use crate::{AtlasImage, BatchError, MAX_ATLAS_DIMENSION, SpriteBatch, TileCoord, TileView};
use oteryn_client_domain::world::{CELL_PX, Draw, DrawCell};
use std::collections::BTreeMap;

/// Builtin cells preceding sprite cells in the static map atlas.
pub const BUILTIN_CELLS: usize = 3;
pub const BLACK_CELL: u16 = 0;
pub const TARGET_CELL: u16 = 1;
pub const MARKER_CELL: u16 = 2;
/// Number of cells in one full atlas row.
pub const ATLAS_COLUMNS: usize = (MAX_ATLAS_DIMENSION / CELL_PX) as usize;
pub const MAX_CELLS: usize = ATLAS_COLUMNS * ATLAS_COLUMNS;

/// Lays `count` 32 px cells, stored one after another, out in rows of [`ATLAS_COLUMNS`].
pub fn atlas_from_cells(cells: Vec<u8>, count: usize) -> Result<AtlasImage, BatchError> {
    let columns = count.min(ATLAS_COLUMNS);
    let rows = count.div_ceil(columns);
    let px = CELL_PX as usize;
    let mut rgba = vec![0; columns * px * rows * px * 4];
    for (index, cell) in cells.chunks_exact(px * px * 4).enumerate() {
        let (column, row) = (index % columns, index / columns);
        for line in 0..px {
            let at = ((row * px + line) * columns * px + column * px) * 4;
            rgba[at..at + px * 4].copy_from_slice(&cell[line * px * 4..(line + 1) * px * 4]);
        }
    }
    AtlasImage::new(CELL_PX, columns as u32, rows as u32, rgba)
}

/// The builtin cells, one after another: black, a yellow outline, a red square.
pub fn builtin_rgba() -> Vec<u8> {
    let px = CELL_PX as usize;
    let mut rgba = Vec::with_capacity(BUILTIN_CELLS * px * px * 4);
    for cell in 0..BUILTIN_CELLS {
        for y in 0..px {
            for x in 0..px {
                let edge = x.min(y).min(px - 1 - x).min(px - 1 - y);
                rgba.extend_from_slice(&match cell {
                    0 => [0, 0, 0, 255],
                    1 if edge < 2 => [255, 220, 40, 255],
                    2 if edge >= 10 => [220, 40, 40, 255],
                    _ => [0, 0, 0, 0],
                });
            }
        }
    }
    rgba
}

/// Static sprite slots and their upload image. Unassigned cells retain the black slot,
/// which simulation omits when projecting a window larger than the atlas capacity.
pub struct WorldAtlas {
    image: AtlasImage,
    slots: BTreeMap<(u32, u32, u32), u16>,
}

impl WorldAtlas {
    /// Deduplicate source cells and read pixels in sprite order, so the asset reader can
    /// decode each sheet once. The caller supplies immutable asset decoding.
    pub fn prepare<'a>(
        cells: impl IntoIterator<Item = &'a DrawCell>,
        mut read_pixels: impl FnMut(u32, u32, u32) -> Result<Vec<u8>, String>,
    ) -> Result<Self, String> {
        let mut slots = BTreeMap::new();
        for cell in cells {
            slots.insert((cell.sprite_id, cell.cell_x, cell.cell_y), 0_u16);
        }
        let mut rgba = builtin_rgba();
        let mut next = BUILTIN_CELLS;
        for (key, slot) in &mut slots {
            if next >= MAX_CELLS {
                break;
            }
            let pixels = read_pixels(key.0, key.1, key.2)
                .map_err(|error| format!("sprite {}: {error}", key.0))?;
            *slot = next as u16;
            rgba.extend_from_slice(&pixels);
            next += 1;
        }
        let image = atlas_from_cells(rgba, next).map_err(|error| format!("atlas: {error}"))?;
        Ok(Self { image, slots })
    }

    /// Resolve an appearance cell to its assigned atlas slot.
    #[must_use]
    pub fn slot(&self, cell: &DrawCell) -> Option<u16> {
        self.slots
            .get(&(cell.sprite_id, cell.cell_x, cell.cell_y))
            .copied()
    }

    /// Transfer the prepared upload image once projection no longer needs slot lookup.
    #[must_use]
    pub fn into_image(self) -> AtlasImage {
        self.image
    }
}

/// Stream deterministic scene draws into one batch, then append the target highlight.
/// The visitor owns painter order; the renderer owns offsets, UVs and batch limits.
pub fn sprite_batch(
    view: &TileView,
    atlas: &AtlasImage,
    visit: impl FnOnce(
        &mut dyn FnMut((i32, i32), Draw) -> Result<(), BatchError>,
    ) -> Result<(), BatchError>,
    target: Option<TileCoord>,
) -> Result<SpriteBatch, BatchError> {
    let mut sprites = SpriteBatch::new();
    visit(&mut |(x, y), draw: Draw| {
        sprites
            .push_offset(
                view,
                atlas,
                TileCoord::new(x, y),
                draw.cell,
                draw.offset,
                CELL_PX,
            )
            .map(|_| ())
    })?;
    if let Some(target) = target {
        sprites.push(view, atlas, target, TARGET_CELL)?;
    }
    Ok(sprites)
}

#[cfg(test)]
mod tests;
