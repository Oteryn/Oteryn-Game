//! Standing outfit composition uses the same verified sheets as the world view.
#[cfg(windows)]
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
#[cfg(windows)]
use oteryn_client_ui::appearance::Preview;
#[cfg(windows)]
use std::path::Path;
#[cfg(windows)]
pub struct Assets {
    index: AppearanceIndex,
    sheets: SpriteSheets,
}
#[cfg(windows)]
impl Assets {
    pub fn load(root: &Path) -> Result<Self, String> {
        use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
        let store = AssetStore::open_pinned(
            &root.join("content/assets/files"),
            &root.join(MANIFEST_PATH),
            MANIFEST_SHA256,
        )
        .map_err(|e| e.to_string())?;
        let catalog = Catalog::load(&store).map_err(|e| e.to_string())?;
        let index = AppearanceIndex::load(&store, &catalog).map_err(|e| e.to_string())?;
        Ok(Self {
            index,
            sheets: SpriteSheets::new(store, catalog),
        })
    }
    pub fn draw(&mut self, preview: &Preview) -> Result<Vec<u8>, String> {
        let outfit = self
            .index
            .outfit(preview.look_type)
            .ok_or("Outfit artwork unavailable")?;
        let mut pixels = vec![32; 96 * 96 * 4];
        for addon in 0..outfit.pattern_height.min(3) {
            if addon != 0 && preview.addons & (1 << (addon - 1)) == 0 {
                continue;
            }
            let at = ((addon * outfit.pattern_width + 2 % outfit.pattern_width) * outfit.layers)
                as usize;
            let base = *outfit.sprite_ids.get(at).ok_or("Missing outfit frame")?;
            if base == 0 {
                continue;
            }
            let mask = if outfit.layers > 1 {
                outfit.sprite_ids.get(at + 1).copied().filter(|id| *id != 0)
            } else {
                None
            };
            let (width, height) = self.sheets.layout(base).map_err(|e| e.to_string())?.cells();
            for y in 0..height {
                for x in 0..width {
                    let mut cell = self
                        .sheets
                        .cell_rgba(base, x, y)
                        .map_err(|e| e.to_string())?;
                    if let Some(mask) = mask {
                        let mask = self
                            .sheets
                            .cell_rgba(mask, x, y)
                            .map_err(|e| e.to_string())?;
                        tint(&mut cell, &mask, preview.colours);
                    }
                    for row in 0..32 {
                        for column in 0..32 {
                            let dx = 64 + (x as i32 + 1 - width as i32) * 32 + column
                                - outfit.displacement_x as i32;
                            let dy = 64 + (y as i32 + 1 - height as i32) * 32 + row
                                - outfit.displacement_y as i32;
                            if !(0..96).contains(&dx) || !(0..96).contains(&dy) {
                                continue;
                            }
                            let src = ((row * 32 + column) * 4) as usize;
                            let dst = ((dy * 96 + dx) * 4) as usize;
                            let alpha = u16::from(cell[src + 3]);
                            // GDI's bitmap surface uses BGR, with an opaque background.
                            for channel in 0..3 {
                                pixels[dst + channel] =
                                    ((u16::from(cell[src + 2 - channel]) * alpha
                                        + u16::from(pixels[dst + channel]) * (255 - alpha))
                                        / 255) as u8;
                            }
                        }
                    }
                }
            }
        }
        Ok(pixels)
    }
}
fn tint(base: &mut [u8], mask: &[u8], colours: [[u8; 3]; 4]) {
    for (pixel, mask) in base
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(mask.as_chunks::<4>().0)
    {
        if mask[3] == 0 {
            continue;
        }
        let part = match mask[..3] {
            [255, 255, 0] => 0,
            [255, 0, 0] => 1,
            [0, 255, 0] => 2,
            [0, 0, 255] => 3,
            _ => continue,
        };
        for channel in 0..3 {
            pixel[channel] =
                (u16::from(pixel[channel]) * u16::from(colours[part][channel]) / 255) as u8;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outfit_mask_tints_all_four_parts_without_changing_alpha() {
        let mut base = [200, 100, 50, 255].repeat(4);
        tint(
            &mut base,
            &[
                255, 255, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255,
            ],
            [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]],
        );
        assert_eq!(
            base,
            [
                200, 0, 0, 255, 0, 100, 0, 255, 0, 0, 50, 255, 200, 100, 50, 255
            ]
        );
    }
}
