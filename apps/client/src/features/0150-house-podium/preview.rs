//! Bounded RGBA composition used by the native podium preview.
pub fn compose(cells: impl IntoIterator<Item = (i32, i32, Vec<u8>)>) -> Vec<u8> {
    let mut image = vec![32; 128 * 128 * 4];
    for pixel in image.as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    for (ox, oy, rgba) in cells.into_iter().take(64) {
        if rgba.len() != 32 * 32 * 4 {
            continue;
        }
        for y in 0..32i32 {
            for x in 0..32i32 {
                let (dx, dy) = (64 + ox + x, 64 + oy + y);
                if !(0..128).contains(&dx) || !(0..128).contains(&dy) {
                    continue;
                }
                let source = ((y * 32 + x) * 4) as usize;
                let target = ((dy * 128 + dx) * 4) as usize;
                let alpha = u32::from(rgba[source + 3]);
                for c in 0..3 {
                    image[target + c] = ((u32::from(rgba[source + c]) * alpha
                        + u32::from(image[target + c]) * (255 - alpha))
                        / 255) as u8;
                }
            }
        }
    }
    image
}
#[cfg(test)]
mod tests {
    #[test]
    fn podium_preview_composites_and_clips_real_sprite_cells() -> Result<(), &'static str> {
        let mut cell = vec![0; 4096];
        cell[..4].copy_from_slice(&[200, 100, 50, 255]);
        let image = super::compose([(0, 0, cell.clone()), (1000, 0, cell)]);
        assert_eq!(
            &image[(64 * 128 + 64) * 4..(64 * 128 + 64) * 4 + 4],
            &[200, 100, 50, 255]
        );
        assert_eq!(&image[..4], &[32, 32, 32, 255]);
        Ok(())
    }
}
