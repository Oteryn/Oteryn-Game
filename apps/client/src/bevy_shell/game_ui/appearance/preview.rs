use super::AppearanceSnapshot;
use crate::play::feature_0025_appearance::view::AppearancePreview;

#[derive(Default)]
pub(super) struct PreviewCache {
    assets: Option<Result<AppearancePreview, String>>,
}
impl PreviewCache {
    pub(super) fn pixels(&mut self, snapshot: &AppearanceSnapshot) -> Result<Vec<u8>, String> {
        self.pixels_with(snapshot, || {
            let root = std::env::var_os("OTERYN_ASSET_DIR")
                .ok_or_else(|| "Appearance artwork unavailable".to_owned())?;
            AppearancePreview::load(std::path::Path::new(&root))
        })
    }
    fn pixels_with(
        &mut self,
        snapshot: &AppearanceSnapshot,
        load: impl FnOnce() -> Result<AppearancePreview, String>,
    ) -> Result<Vec<u8>, String> {
        self.assets
            .get_or_insert_with(load)
            .as_mut()
            .map_err(|error| error.clone())?
            .pixels(snapshot)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_preview_assets_load_once_across_rebuilds()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, _, outbox) = crate::play::play_channel();
        crate::play::feature_0025_appearance::view::bevy_window_tests::publish(
            &outbox,
            crate::play::feature_0025_appearance::view::bevy_window_tests::catalogue(),
        );
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut cache = PreviewCache::default();
        let mut loads = 0;
        let mut first = Vec::new();
        for iteration in 0..2 {
            let pixels = cache.pixels_with(&link.appearance_snapshot(), || {
                loads += 1;
                AppearancePreview::load(&root)
            })?;
            assert_eq!(pixels.len(), 128 * 128 * 4);
            if iteration == 0 {
                first = pixels;
                assert!(link.appearance_colour(0));
            } else {
                assert_ne!(pixels, first);
            }
        }
        assert_eq!(loads, 1);
        Ok(())
    }
}
