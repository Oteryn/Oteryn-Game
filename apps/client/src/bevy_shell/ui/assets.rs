//! Validate the external shell skin assets.
use std::path::Path;

const FILES: [&str; 7] = [
    "images/skin/classic/button-grey-up.png",
    "images/skin/classic/button-grey-down.png",
    "images/skin/classic/checkbox-checked.png",
    "images/skin/classic/checkbox-unchecked.png",
    "images/skin/classic/dialog-frame-borderimage.png",
    "images/1pixel-down-frame.png",
    "fonts/verdanab.ttf",
];
pub(in crate::bevy_shell) fn validate_assets(root: &Path) -> Result<(), String> {
    for name in FILES {
        let path = root
            .join(name)
            .canonicalize()
            .map_err(|_| format!("Missing external UI asset: {name}"))?;
        if !path.starts_with(root) || !path.is_file() {
            return Err(format!("UI asset leaves --ui-assets directory: {name}"));
        }
    }
    Ok(())
}
