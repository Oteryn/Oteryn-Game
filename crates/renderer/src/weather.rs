use super::AtlasImage;
/// Blend a visual weather overlay into RGB, preserving alpha and atlas geometry.
pub fn weather_atlas(atlas: &AtlasImage, color: [u8; 3], strength: u8) -> AtlasImage {
    let mut weather = atlas.clone();
    for pixel in weather.rgba.as_chunks_mut::<4>().0 {
        for channel in 0..3 {
            pixel[channel] = ((u16::from(pixel[channel]) * (255 - u16::from(strength))
                + u16::from(color[channel]) * u16::from(strength))
                / 255) as u8;
        }
    }
    weather
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weather_preserves_alpha_geometry_and_source() -> Result<(), crate::BatchError> {
        let atlas = AtlasImage::new(1, 1, 1, vec![200, 100, 50, 123])?;
        assert_eq!(weather_atlas(&atlas, [0; 3], 0), atlas);
        assert_eq!(
            weather_atlas(&atlas, [0, 255, 100], 255).rgba(),
            &[0, 255, 100, 123]
        );
        let weather = weather_atlas(&atlas, [0; 3], 128);
        assert_eq!(weather.rgba(), &[99, 49, 24, 123]);
        assert_eq!((weather.width(), weather.height()), (1, 1));
        assert_eq!(atlas.rgba(), &[200, 100, 50, 123]);
        Ok(())
    }
}
