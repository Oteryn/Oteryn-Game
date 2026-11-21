//! Digest-bound local mount presentation metadata; never an ownership source.
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read, path::Path};
pub(crate) fn load(root: &Path) -> Result<([u8; 32], BTreeMap<String, u32>), String> {
    // Presentation metadata is accepted only when its normalized document matches the server's digest.
    // It never adds choices to the server-owned catalogue or grants an entitlement.
    // Match the native cosmetics owner's 16 MiB input bound. The source
    // authoring packet includes provenance for the full mount catalogue.
    const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;
    let mut bytes = Vec::new();
    std::fs::File::open(root.join("content/cosmetics/outfits/authoring.json"))
        .map_err(|e| e.to_string())?
        .take(MAX_METADATA_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_METADATA_BYTES {
        return Err("Appearance metadata exceeds its bound".into());
    }
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let document = value
        .get_mut("document")
        .ok_or("Appearance metadata unavailable")?;
    document["schema"] = "OTERYN_CHARACTER_COSMETICS_CANDIDATE/v1".into();
    let normalized = serde_json::to_vec(document).map_err(|e| e.to_string())?;
    let digest: [u8; 32] = Sha256::digest(&normalized).into();
    let entries = document["mounts"]
        .as_array()
        .filter(|items| items.len() <= 256)
        .ok_or("Mount metadata unavailable")?;
    let mut mounts = BTreeMap::new();
    for entry in entries {
        let key = entry["key"].as_str().ok_or("Mount key unavailable")?;
        let look = entry["look"]["id"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n != 0)
            .ok_or("Mount artwork unavailable")?;
        mounts.insert(key.to_owned(), look);
    }
    Ok((digest, mounts))
}
