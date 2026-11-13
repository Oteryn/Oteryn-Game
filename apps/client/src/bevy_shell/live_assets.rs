//! Admission-only metadata checks; sprite pixels remain verified lazily by the asset owner.
use oteryn_client_assets::world::{MANIFEST_PATH, MANIFEST_SHA256};
use oteryn_client_assets::{AppearanceIndex, AssetStore, Catalog};
use std::{ffi::OsString, path::PathBuf};

pub(super) fn preflight(root: Option<OsString>) -> Result<(), String> {
    let failure = |reason: &str| {
        format!(
            "Cannot start sign-in: {reason}. Set OTERYN_ASSET_DIR explicitly to the pinned Oteryn asset root."
        )
    };
    let supplied = root
        .filter(|value| !value.is_empty())
        .ok_or_else(|| failure("live asset root is missing"))?;
    let root = PathBuf::from(supplied)
        .canonicalize()
        .map_err(|_| failure("live asset root cannot be opened"))?;
    if !root.is_dir() {
        return Err(failure("live asset root is not a directory"));
    }
    let store = AssetStore::open_pinned(
        &root.join("content/assets/files"),
        &root.join(MANIFEST_PATH),
        MANIFEST_SHA256,
    )
    .map_err(|_| failure("live asset manifest is missing or does not match this client"))?;
    let catalog =
        Catalog::load(&store).map_err(|_| failure("pinned live asset catalog cannot be loaded"))?;
    AppearanceIndex::load(&store, &catalog)
        .map_err(|_| failure("pinned live appearances cannot be loaded"))?;
    Ok(())
}
