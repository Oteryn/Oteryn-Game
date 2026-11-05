use super::AtlasImage;
/// Multiply authored atlas RGB by ambient RGB, preserving alpha and atlas layout.
pub fn lit_atlas(atlas: &AtlasImage, ambient: [u8; 3]) -> AtlasImage {
    let mut lit = atlas.clone();
    for pixel in lit.rgba.as_chunks_mut::<4>().0 {
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
}
