//! Official outfit base/addon sprites tinted by the already validated server palette.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutfitRgbaCell {
    pub offset: [i32; 2],
    pub rgba: Vec<u8>,
}
impl AppearanceIndex {
    /// Resolve standing base plus the two addon patterns. RGB comes from the
    /// negotiated Catalogue; this layer never guesses a palette from an index.
    pub fn resolve_outfit_rgba(
        &self,
        sheets: &mut SpriteSheets,
        look_type: u32,
        direction: u32,
        addons: u8,
        colours: [[u8; 3]; 4],
    ) -> Result<Vec<OutfitRgbaCell>, AssetError> {
        let appearance = self
            .outfits
            .get(&look_type)
            .ok_or(AssetError::UnknownAppearance { id: look_type })?;
        if addons & !3 != 0 {
            return Err(invalid_outfit("addon mask"));
        }
        let mut output = Vec::new();
        for pattern in 0..3 {
            if pattern != 0 && addons & (1 << (pattern - 1)) == 0 {
                continue;
            }
            if pattern >= appearance.pattern_height {
                return Err(invalid_outfit("addon pattern unavailable"));
            }
            let position = (direction % appearance.pattern_width, pattern, 0);
            let bases = entry(appearance, sheets, position, 0..1)?.cells;
            let masks = if appearance.layers > 1 {
                entry(appearance, sheets, position, 1..2)?.cells
            } else {
                Vec::new()
            };
            for cell in bases {
                let mut rgba = sheets.cell_rgba(cell.sprite_id, cell.cell_x, cell.cell_y)?;
                if let Some(mask) = masks
                    .iter()
                    .find(|mask| mask.offset_x == cell.offset_x && mask.offset_y == cell.offset_y)
                {
                    let mask = sheets.cell_rgba(mask.sprite_id, mask.cell_x, mask.cell_y)?;
                    if mask.len() != rgba.len() {
                        return Err(invalid_outfit("mask dimensions"));
                    }
                    tint_outfit(&mut rgba, &mask, colours);
                }
                output.push(OutfitRgbaCell {
                    offset: [cell.offset_x, cell.offset_y],
                    rgba,
                });
                if output.len() > MAX_ENTRY_CELLS {
                    return Err(AssetError::TooManyCells {
                        cells: output.len(),
                    });
                }
            }
        }
        Ok(output)
    }
}
fn invalid_outfit(reason: &str) -> AssetError {
    AssetError::Malformed {
        file: "official outfit appearance".into(),
        reason: reason.into(),
    }
}
fn tint_outfit(base: &mut [u8], mask: &[u8], colours: [[u8; 3]; 4]) {
    for (pixel, mask) in base
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(mask.as_chunks::<4>().0.iter())
    {
        if pixel[3] == 0 || mask[3] == 0 {
            continue;
        }
        let colour = match [mask[0], mask[1], mask[2]] {
            [255, 255, 0] => colours[0], // head
            [255, 0, 0] => colours[1],   // body
            [0, 255, 0] => colours[2],   // legs
            [0, 0, 255] => colours[3],   // feet
            _ => continue,
        };
        for component in 0..3 {
            pixel[component] =
                ((u16::from(pixel[component]) * u16::from(colour[component])) / 255) as u8;
        }
    }
}
