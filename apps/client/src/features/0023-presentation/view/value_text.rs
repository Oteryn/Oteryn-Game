use super::{AtlasImage, BatchError, SpriteBatch, TileView, pixel};
use oteryn_session::PresentationValue;

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

pub(super) fn number(
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
