//! Small bitmap labels in the existing sprite batch; no platform text surface is required.
use crate::{AtlasImage, BatchError, MAX_BATCH_QUADS, QuadInstance, SpriteBatch};

/// Append a screen-space label quad using the same bounded buffer as world sprites.
pub fn label_quad(
    batch: &mut SpriteBatch,
    atlas: &AtlasImage,
    cell: u16,
    position: [f32; 2],
) -> Result<(), BatchError> {
    label_quad_scaled(batch, atlas, cell, position, 100)
}

/// UI scale changes label geometry without changing the atlas or world zoom.
pub fn label_quad_scaled(
    batch: &mut SpriteBatch,
    atlas: &AtlasImage,
    cell: u16,
    position: [f32; 2],
    percentage: u16,
) -> Result<(), BatchError> {
    if !(1..=200).contains(&percentage) {
        return Err(BatchError::InvalidView);
    }
    if batch.instances.len() >= MAX_BATCH_QUADS {
        return Err(BatchError::CapacityExceeded {
            max: MAX_BATCH_QUADS,
        });
    }
    batch.instances.push(QuadInstance {
        position,
        size: [8.0, 12.0].map(|value| value * f32::from(percentage) / 100.0),
        uv: atlas.uv_rect(cell)?,
    });
    Ok(())
}

/// Original 5 by 7 bitmap alphabet. Unsupported Unicode uses a visible question mark.
pub fn glyph_rgba(character: char) -> Vec<u8> {
    let rows: [u8; 7] = match character {
        'a' => [0, 0, 14, 1, 15, 17, 15],
        'b' => [16, 16, 30, 17, 17, 17, 30],
        'c' => [0, 0, 14, 16, 16, 17, 14],
        'd' => [1, 1, 15, 17, 17, 17, 15],
        'e' => [0, 0, 14, 17, 31, 16, 14],
        'f' => [6, 9, 8, 28, 8, 8, 8],
        'g' => [0, 15, 17, 17, 15, 1, 14],
        'h' => [16, 16, 30, 17, 17, 17, 17],
        'i' => [4, 0, 12, 4, 4, 4, 14],
        'j' => [2, 0, 6, 2, 2, 18, 12],
        'k' => [16, 16, 18, 20, 24, 20, 18],
        'l' => [12, 4, 4, 4, 4, 4, 14],
        'm' => [0, 0, 26, 21, 21, 17, 17],
        'n' => [0, 0, 30, 17, 17, 17, 17],
        'o' => [0, 0, 14, 17, 17, 17, 14],
        'p' => [0, 30, 17, 17, 30, 16, 16],
        'q' => [0, 15, 17, 17, 15, 1, 1],
        'r' => [0, 0, 22, 25, 16, 16, 16],
        's' => [0, 0, 15, 16, 14, 1, 30],
        't' => [8, 8, 28, 8, 8, 9, 6],
        'u' => [0, 0, 17, 17, 17, 19, 13],
        'v' => [0, 0, 17, 17, 17, 10, 4],
        'w' => [0, 0, 17, 17, 21, 21, 10],
        'x' => [0, 0, 17, 10, 4, 10, 17],
        'y' => [0, 17, 17, 17, 15, 1, 14],
        'z' => [0, 0, 31, 2, 4, 8, 31],
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 14],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        ' ' => [0; 7],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        '\'' => [4, 4, 0, 0, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 6, 6],
        _ => [14, 17, 1, 2, 4, 0, 4],
    };
    let mut pixels = vec![0; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let (gx, gy) = (x * 6 / 32, y * 8 / 32);
            let at = (y * 32 + x) * 4;
            if gx < 5 && gy < 7 && rows[gy] & (1 << (4 - gx)) != 0 {
                pixels[at..at + 4].copy_from_slice(&[240, 255, 230, 255]);
            } else {
                pixels[at..at + 4].copy_from_slice(&[0, 0, 0, 180]);
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labels_use_distinct_readable_glyphs_and_bounded_quads() -> Result<(), BatchError> {
        assert_ne!(glyph_rgba('R'), glyph_rgba('a'));
        let atlas = AtlasImage::new(32, 1, 1, glyph_rgba('R'))?;
        let mut batch = SpriteBatch::new();
        label_quad(&mut batch, &atlas, 0, [10.0, 20.0])?;
        assert_eq!(batch.instances()[0].size, [8.0, 12.0]);
        assert!(label_quad(&mut batch, &atlas, 1, [0.0, 0.0]).is_err());
        Ok(())
    }
}
