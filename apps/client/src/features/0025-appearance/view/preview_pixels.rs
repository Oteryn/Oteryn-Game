//! Bounded CPU composition of outfit base, colour masks, addons and mount sprites.
use super::{Image, PreviewAssets};
impl PreviewAssets {
    pub(super) fn draw_look(
        &mut self,
        look: u32,
        addons: u8,
        colours: Option<[[u8; 3]; 4]>,
        mounted: bool,
        image: &mut Image,
    ) -> Result<(), String> {
        let appearance = self
            .index
            .outfit(look)
            .ok_or("Appearance artwork unavailable")?;
        let depth = u32::from(mounted && appearance.pattern_depth > 1);
        let mut sprites = Vec::new();
        for pattern in 0..appearance.pattern_height.min(3) {
            if pattern != 0 && addons & (1 << (pattern - 1)) == 0 {
                continue;
            }
            let index = (((depth * appearance.pattern_height + pattern) * appearance.pattern_width
                + 2 % appearance.pattern_width)
                * appearance.layers) as usize;
            let sprite = *appearance
                .sprite_ids
                .get(index)
                .ok_or("Appearance frame unavailable")?;
            let mask = (colours.is_some() && appearance.layers > 1)
                .then(|| appearance.sprite_ids.get(index + 1).copied())
                .flatten()
                .filter(|id| *id != 0);
            sprites.push((sprite, mask));
        }
        let shift = [
            appearance.displacement_x as i32,
            appearance.displacement_y as i32,
        ];
        for (sprite, mask) in sprites {
            if sprite != 0 {
                self.draw_sprite(sprite, mask, colours, shift, image)?;
            }
        }
        Ok(())
    }
    fn draw_sprite(
        &mut self,
        sprite: u32,
        mask: Option<u32>,
        colours: Option<[[u8; 3]; 4]>,
        shift: [i32; 2],
        image: &mut Image,
    ) -> Result<(), String> {
        let (width, height) = self
            .sheets
            .layout(sprite)
            .map_err(|e| e.to_string())?
            .cells();
        for cy in 0..height {
            for cx in 0..width {
                let base = self
                    .sheets
                    .cell_rgba(sprite, cx, cy)
                    .map_err(|e| e.to_string())?;
                let mask = mask
                    .map(|id| self.sheets.cell_rgba(id, cx, cy))
                    .transpose()
                    .map_err(|e| e.to_string())?;
                for (i, source) in base.as_chunks::<4>().0.iter().enumerate() {
                    if source[3] == 0 {
                        continue;
                    }
                    let x = 96 - width as i32 * 32 + cx as i32 * 32 + (i % 32) as i32 - shift[0];
                    let y = 96 - height as i32 * 32 + cy as i32 * 32 + (i / 32) as i32 - shift[1];
                    if !(0..128).contains(&x) || !(0..128).contains(&y) {
                        continue;
                    }
                    let mut rgb = [source[0], source[1], source[2]];
                    if let (Some(mask), Some(colours)) = (&mask, colours) {
                        let m = &mask[i * 4..i * 4 + 4];
                        let part = match (m[0] != 0, m[1] != 0, m[2] != 0) {
                            (true, true, false) => Some(0),
                            (true, false, false) => Some(1),
                            (false, true, false) => Some(2),
                            (false, false, true) => Some(3),
                            _ => None,
                        };
                        if m[3] != 0
                            && let Some(part) = part
                        {
                            for (pixel, colour) in rgb.iter_mut().zip(colours[part]) {
                                *pixel = (u32::from(*pixel) * u32::from(colour) / 255) as u8;
                            }
                        }
                    }
                    let destination = &mut image.rgba[(y as usize * 128 + x as usize) * 4..][..4];
                    let alpha = u32::from(source[3]);
                    for (target, source) in destination[..3].iter_mut().zip(rgb) {
                        *target = ((u32::from(source) * alpha + u32::from(*target) * (255 - alpha))
                            / 255) as u8;
                    }
                }
            }
        }
        Ok(())
    }
}
