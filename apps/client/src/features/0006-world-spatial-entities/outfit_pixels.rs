use oteryn_client_assets::SpriteSheets;
pub(super) struct Outfit<'a> {
    pub pattern: [u32; 4],
    pub displacement: [u32; 2],
    pub sprites: &'a [u32],
}
type Cell = ([i32; 2], Vec<u8>);
pub(super) fn resolve(
    a: Outfit<'_>,
    sheets: &mut SpriteSheets,
    direction: u32,
    phase: u32,
    colours: [u16; 4],
    addons: u8,
    palette: &[[u8; 3]],
) -> Option<Vec<Cell>> {
    let mut cells = Vec::new();
    for addon in 0..a.pattern[1].min(3) {
        if addon != 0 && addons & (1 << (addon - 1)) == 0 {
            continue;
        }
        let index = ((phase * a.pattern[2] * a.pattern[1] + addon) * a.pattern[0]
            + direction % a.pattern[0])
            * a.pattern[3];
        let sprite = *a.sprites.get(index as usize)?;
        if sprite == 0 {
            continue;
        }
        let mask = if a.pattern[3] > 1 {
            *a.sprites.get(index as usize + 1)?
        } else {
            0
        };
        let (width, height) = sheets.layout(sprite).ok()?.cells();
        for y in 0..height {
            for x in 0..width {
                let mut rgba = sheets.cell_rgba(sprite, x, y).ok()?;
                if mask != 0 && !palette.is_empty() {
                    let mask = sheets.cell_rgba(mask, x, y).ok()?;
                    for (pixel, mask) in rgba
                        .as_chunks_mut::<4>()
                        .0
                        .iter_mut()
                        .zip(mask.as_chunks::<4>().0)
                    {
                        let slot = match (mask[0] > 0, mask[1] > 0, mask[2] > 0) {
                            (true, true, false) => Some(0),
                            (true, false, false) => Some(1),
                            (false, true, false) => Some(2),
                            (false, false, true) => Some(3),
                            _ => None,
                        };
                        if mask[3] > 0
                            && let Some(rgb) =
                                slot.and_then(|slot| palette.get(usize::from(colours[slot])))
                        {
                            for channel in 0..3 {
                                pixel[channel] = (u16::from(pixel[channel])
                                    * u16::from(rgb[channel])
                                    / 255) as u8;
                            }
                        }
                    }
                }
                cells.push((
                    [
                        (x as i32 + 1 - width as i32) * 32 - a.displacement[0] as i32,
                        (y as i32 + 1 - height as i32) * 32 - a.displacement[1] as i32,
                    ],
                    rgba,
                ));
            }
        }
    }
    Some(cells)
}
