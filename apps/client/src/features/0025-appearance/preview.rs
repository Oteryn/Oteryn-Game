//! Timed outfit composition uses the same verified sheets as the world view.
#[cfg(any(windows, feature = "bevy-shell"))]
use crate::appearance_mount_binding as mount_binding;
#[cfg(any(windows, feature = "bevy-shell"))]
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog, SpriteSheets};
#[cfg(windows)]
use oteryn_client_ui::appearance::Preview;
#[cfg(any(windows, feature = "bevy-shell"))]
use std::path::Path;
#[cfg(any(windows, feature = "bevy-shell"))]
pub struct Assets {
    index: AppearanceIndex,
    sheets: SpriteSheets,
    mounts: Option<([u8; 32], std::collections::BTreeMap<String, u32>)>,
}
#[cfg(any(windows, feature = "bevy-shell"))]
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
            mounts: mount_binding::load(root).ok(),
            sheets: SpriteSheets::new(store, catalog),
        })
    }
    #[cfg(windows)]
    pub fn draw(&mut self, preview: &Preview) -> Result<Vec<u8>, String> {
        let mut pixels = vec![32; 96 * 96 * 4];
        self.draw_look(
            preview.look_type,
            preview.addons,
            Some(preview.colours),
            false,
            (2, 0, false),
            &mut pixels,
        )?;
        Ok(pixels)
    }
    pub fn draw_panel(
        &mut self,
        panel: &oteryn_client_ui::appearance::Panel,
        direction: u32,
        elapsed_ms: u64,
        moving: bool,
    ) -> Result<Vec<u8>, String> {
        let preview = panel.preview().ok_or("Choose an outfit")?;
        let mount = panel.fields[2].trim();
        let mut pixels = vec![32; 96 * 96 * 4];
        if !mount.is_empty() {
            let catalogue = panel
                .catalogue
                .as_ref()
                .ok_or("Waiting for appearance choices")?;
            if !catalogue.mounts.iter().any(|key| key == mount) {
                return Err("Choose an owned mount".into());
            }
            let (_, mounts) = self
                .mounts
                .as_ref()
                .filter(|(digest, _)| *digest == catalogue.context.cosmetics_digest)
                .ok_or("Mount artwork needs updating")?;
            let look = *mounts.get(mount).ok_or("Mount artwork unavailable")?;
            self.draw_look(
                look,
                0,
                None,
                false,
                (direction, elapsed_ms, moving),
                &mut pixels,
            )?;
        }
        self.draw_look(
            preview.look_type,
            preview.addons,
            Some(preview.colours),
            !mount.is_empty(),
            (direction, elapsed_ms, moving),
            &mut pixels,
        )?;
        Ok(pixels)
    }
    fn draw_look(
        &mut self,
        look: u32,
        addons: u8,
        colours: Option<[[u8; 3]; 4]>,
        mounted: bool,
        (direction, elapsed_ms, moving): (u32, u64, bool),
        pixels: &mut [u8],
    ) -> Result<(), String> {
        let outfit = self
            .index
            .outfit_preview(look, moving)
            .ok_or("Outfit artwork unavailable")?;
        for addon in 0..outfit.pattern_height.min(3) {
            if addon != 0 && addons & (1 << (addon - 1)) == 0 {
                continue;
            }
            let at = frame_index(
                [
                    outfit.pattern_width,
                    outfit.pattern_height,
                    outfit.pattern_depth,
                    outfit.layers,
                ],
                &outfit.phase_ms,
                addon,
                direction,
                elapsed_ms,
                mounted,
            );
            let base = *outfit.sprite_ids.get(at).ok_or("Missing outfit frame")?;
            if base == 0 {
                continue;
            }
            let mask = if colours.is_some() && outfit.layers > 1 {
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
                    if let (Some(mask), Some(colours)) = (mask, colours) {
                        let mask = self
                            .sheets
                            .cell_rgba(mask, x, y)
                            .map_err(|e| e.to_string())?;
                        tint(&mut cell, &mask, colours);
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
        Ok(())
    }
}
#[cfg(any(windows, feature = "bevy-shell"))]
fn frame_index(
    [width, height, depth, layers]: [u32; 4],
    phase_ms: &[u32],
    addon: u32,
    direction: u32,
    elapsed_ms: u64,
    mounted: bool,
) -> usize {
    let cycle: u64 = phase_ms.iter().map(|ms| u64::from(*ms)).sum();
    let mut remaining = elapsed_ms % cycle.max(1);
    let phase = phase_ms
        .iter()
        .position(|ms| {
            if remaining < u64::from(*ms) {
                true
            } else {
                remaining -= u64::from(*ms);
                false
            }
        })
        .unwrap_or(0) as u32;
    ((((phase * depth + u32::from(mounted && depth > 1)) * height + addon) * width
        + direction % width)
        * layers) as usize
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
    #[cfg(any(windows, feature = "bevy-shell"))]
    #[test]
    fn preview_frames_follow_direction_duration_and_full_pattern_stride() {
        let frame_index = |addon, direction, elapsed| {
            super::frame_index([4, 3, 2, 2], &[100, 200], addon, direction, elapsed, false)
        };
        assert_eq!(frame_index(1, 0, 0), 8);
        assert_eq!(frame_index(1, 1, 99), 10);
        assert_eq!(frame_index(1, 1, 100), 58);
        assert_eq!(frame_index(1, 1, 299), 58);
        assert_eq!(frame_index(1, 1, 300), 10);
        assert_eq!(
            super::frame_index([4, 3, 2, 2], &[100, 200], 1, 1, 100, true),
            82
        );
    }
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
