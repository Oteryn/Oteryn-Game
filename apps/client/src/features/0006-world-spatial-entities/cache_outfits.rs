// Standing outfit cells: base and selected addons, each tinted by its own mask.
type OutfitCell = ([i32; 2], Vec<u8>);
type OutfitKey = (u32, u32, [u16; 4], u8);

impl PresentationCache {
    pub(crate) fn set_palette(&mut self, palette: &[[u8; 3]]) -> bool {
        if self.palette == palette {
            return false;
        }
        self.palette.clear();
        self.palette.extend_from_slice(palette);
        // Previously packed cells remain valid for retained scene snapshots.
        self.outfits.clear();
        true
    }
}

fn outfit_cells(
    assets: &mut Assets,
    look_type: u32,
    direction: u32,
    addons: u8,
    colours: Option<[[u8; 3]; 4]>,
) -> Option<Vec<OutfitCell>> {
    let appearance = assets.index.outfit(look_type)?;
    let sheets = &mut assets.sheets;
    let mut cells = Vec::new();
    for pattern in 0..appearance.pattern_height.min(3) {
        if pattern > 0 && addons & (1 << (pattern - 1)) == 0 {
            continue;
        }
        let at = ((pattern * appearance.pattern_width + direction % appearance.pattern_width)
            * appearance.layers) as usize;
        let sprite = *appearance.sprite_ids.get(at)?;
        if sprite == 0 {
            continue;
        }
        let mask = if appearance.layers > 1 {
            appearance
                .sprite_ids
                .get(at + 1)
                .copied()
                .filter(|id| *id != 0)
        } else {
            None
        };
        let (width, height) = sheets.layout(sprite).ok()?.cells();
        for cy in 0..height {
            for cx in 0..width {
                let mut rgba = sheets.cell_rgba(sprite, cx, cy).ok()?;
                if let (Some(mask), Some(colours)) = (mask, colours) {
                    let mask = sheets.cell_rgba(mask, cx, cy).ok()?;
                    tint(&mut rgba, &mask, colours);
                }
                cells.push((
                    [
                        (cx as i32 + 1 - width as i32) * 32 - appearance.displacement_x as i32,
                        (cy as i32 + 1 - height as i32) * 32 - appearance.displacement_y as i32,
                    ],
                    rgba,
                ));
            }
        }
    }
    Some(cells)
}

fn tint(rgba: &mut [u8], mask: &[u8], colours: [[u8; 3]; 4]) {
    for (pixel, mask) in rgba
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(mask.as_chunks::<4>().0)
    {
        if mask[3] == 0 {
            continue;
        }
        let part = match (mask[0], mask[1], mask[2]) {
            (255, 255, 0) => 0,
            (255, 0, 0) => 1,
            (0, 255, 0) => 2,
            (0, 0, 255) => 3,
            _ => continue,
        };
        for (channel, colour) in pixel[..3].iter_mut().zip(colours[part]) {
            *channel = (u16::from(*channel) * u16::from(colour) / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tint_tests {
    use super::*;
    #[test]
    fn spatial_outfit_masks_multiply_each_body_part_and_preserve_alpha() {
        let mut pixels = [200, 100, 50, 128].repeat(6);
        let masks = [
            [255, 255, 0, 255],
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 0, 0, 0],
            [0, 0, 0, 255],
        ]
        .concat();
        tint(
            &mut pixels,
            &masks,
            [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]],
        );
        assert_eq!(
            pixels,
            [
                [200, 0, 0, 128],
                [0, 100, 0, 128],
                [0, 0, 50, 128],
                [200, 100, 50, 128],
                [200, 100, 50, 128],
                [200, 100, 50, 128],
            ]
            .concat()
        );
    }
}
