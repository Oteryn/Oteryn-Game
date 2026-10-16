//! Unregistered APP-4 preview consumer. Produces pixels only; no UI/session/unlock state.
use oteryn_client_runtime::appearance_rendering::{
    AppearanceRenderError, ComposedOutfit, OutfitPixels, PinnedOutfitAssets,
};

/// A fresh render result, never a retained old look masquerading as the current preview.
#[derive(Debug)]
pub enum OutfitPreview {
    Resolved(ComposedOutfit),
    Unresolved(AppearanceRenderError),
}

/// The very same native asset compositor is callable by future scene adoption. Preview
/// cannot select a sex, grant an unlock, store a selection or alter an observed actor.
pub fn render_outfit_preview(
    assets: &mut PinnedOutfitAssets,
    look: OutfitPixels,
    palette: &[[u8; 3]],
) -> OutfitPreview {
    match assets.compose(look, palette) {
        Ok(pixels) => OutfitPreview::Resolved(pixels),
        Err(error) => OutfitPreview::Unresolved(error),
    }
}

#[cfg(test)]
#[path = "outfit_preview_tests.rs"]
mod tests;
