use super::AtlasImage;
use oteryn_client_domain::world::CELL_PX;
/// Multiply authored RGB by ambient RGB, preserving alpha, layout and the UI palette.
pub fn lit_atlas(atlas: &AtlasImage, ambient: [u8; 3]) -> AtlasImage {
    let mut lit = atlas.clone();
    let width = atlas.width() as usize;
    let px = atlas.cell_px() as usize;
    let palette = crate::world::PALETTE_CELL as usize;
    let columns = width / px;
    for (index, pixel) in lit.rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        // World overlays use the builtin palette as emissive UI pixels.
        if px == CELL_PX as usize
            && atlas.cell_count() > palette as u32
            && (index / width / px) * columns + index % width / px == palette
        {
            continue;
        }
        for channel in 0..3 {
            pixel[channel] = (u16::from(pixel[channel]) * u16::from(ambient[channel]) / 255) as u8;
        }
    }
    lit
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ambient_changes_rgb_and_preserves_alpha_and_layout() -> Result<(), crate::BatchError> {
        let atlas = AtlasImage::new(1, 1, 1, vec![200, 100, 50, 123])?;
        let lit = lit_atlas(&atlas, [128, 255, 0]);
        assert_eq!(lit.rgba(), &[100, 100, 0, 123]);
        assert_eq!((lit.width(), lit.height()), (1, 1));
        assert_eq!(atlas.rgba(), &[200, 100, 50, 123]);
        Ok(())
    }
    #[test]
    fn world_palette_remains_visible_in_darkness() -> Result<(), crate::BatchError> {
        use crate::world::{BUILTIN_CELLS, PALETTE_CELL, atlas_from_cells, builtin_rgba};
        let atlas = atlas_from_cells(builtin_rgba(), BUILTIN_CELLS)?;
        let lit = lit_atlas(&atlas, [0, 0, 0]);
        let x = usize::from(PALETTE_CELL) * 32;
        for y in 0..32 {
            let at = (y * atlas.width() as usize + x) * 4;
            assert_eq!(&lit.rgba()[at..at + 128], &atlas.rgba()[at..at + 128]);
        }
        Ok(())
    }
}
