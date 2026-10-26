//! Static map atlas layout and builtin cells, independent of asset decoding.

use crate::{AtlasImage, BatchError, MAX_ATLAS_DIMENSION};
use oteryn_client_domain::world::CELL_PX;

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
