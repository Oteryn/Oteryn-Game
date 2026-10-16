use super::*;
use oteryn_client_runtime::appearance_rendering::{OutfitDirection, OutfitPixels};
use std::error::Error;
use std::path::PathBuf;
type TestResult = Result<(), Box<dyn Error>>;
fn assets() -> Result<PinnedOutfitAssets, AppearanceRenderError> {
    let r = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    PinnedOutfitAssets::open(
        &r.join("content/assets/files"),
        &r.join("imports/official/client-assets/15.30/manifest.json"),
        "febaff9f4bd7e0f8a029736e446a81f1626805e895e7c2268018e0a9a8493fe4",
    )
}
fn look() -> OutfitPixels {
    OutfitPixels {
        look_type: 128,
        direction: OutfitDirection::South,
        colours: [0; 4],
        addons: 3,
    }
}
#[test]
fn native_preview_is_the_actual_shared_compositor_consumer() -> TestResult {
    let mut assets = assets()?;
    let palette = [[255; 3]; 133]; // synthetic identity palette
    let expected = assets.compose(look(), &palette)?;
    match render_outfit_preview(&mut assets, look(), &palette) {
        OutfitPreview::Resolved(actual) => assert_eq!(actual, expected),
        OutfitPreview::Unresolved(error) => return Err(error.into()),
    }
    Ok(())
}
#[test]
fn native_preview_failure_is_explicit_and_does_not_reuse_previous_frame() -> TestResult {
    let mut assets = assets()?;
    let palette = [[255; 3]; 133];
    assert!(matches!(
        render_outfit_preview(&mut assets, look(), &palette),
        OutfitPreview::Resolved(_)
    ));
    assert!(matches!(
        render_outfit_preview(
            &mut assets,
            OutfitPixels {
                look_type: 0,
                ..look()
            },
            &palette
        ),
        OutfitPreview::Unresolved(AppearanceRenderError::UnknownLook)
    ));
    assert!(matches!(
        render_outfit_preview(&mut assets, look(), &palette[..132]),
        OutfitPreview::Unresolved(AppearanceRenderError::InvalidPalette)
    ));
    assert!(matches!(
        render_outfit_preview(&mut assets, look(), &palette),
        OutfitPreview::Resolved(_)
    ));
    Ok(())
}
