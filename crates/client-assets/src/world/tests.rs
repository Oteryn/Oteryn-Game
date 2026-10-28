use super::*;
use crate::AssetStore;
use sha2::{Digest, Sha256};

fn repo() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_pinned_placement_index_digest_matches_the_repository() -> Result<(), String> {
    let bytes = std::fs::read(repo().join(PLACEMENT_INDEX_PATH)).map_err(|e| e.to_string())?;
    assert_eq!(text_sha256(&bytes), PLACEMENT_INDEX_SHA256);
    Ok(())
}

#[test]
fn a_placement_index_off_the_pin_is_refused() -> Result<(), String> {
    let dir = std::env::temp_dir().join(format!("oteryn-index-pin-{}", std::process::id()));
    let index = dir.join(PLACEMENT_INDEX_PATH);
    std::fs::create_dir_all(index.parent().ok_or("no parent")?).map_err(|e| e.to_string())?;
    std::fs::write(&index, br#"{"regions":[]}"#).map_err(|e| e.to_string())?;
    let result = read_placement_index(&dir);
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    let error = result.err().ok_or("a substituted placement index loaded")?;
    assert!(error.contains("placement index: sha256"), "{error}");
    Ok(())
}

#[test]
fn a_palette_index_outside_the_placement_index_is_refused() {
    let palette = [PaletteEntry { source_item_id: 7 }];
    let found = palette_entry(&palette, 0, "region");
    assert_eq!(found.map(|entry| entry.source_item_id), Ok(7));
    assert_eq!(
        palette_entry(&palette, 1, "region").err().as_deref(),
        Some("region: palette index 1 outside the placement index")
    );
}

#[test]
fn a_region_whose_header_names_another_region_is_refused() {
    let header = b"OTRB\x01\x06\x7e\x00\x7d\x00\x01\x00";
    let decode = |region| {
        decode_region(
            header,
            region,
            (0, 0, 0, 0),
            &mut Budget {
                tiles: 1,
                entries: 1,
            },
        )
    };
    assert_eq!(
        decode((7, 126, 125)).err().as_deref(),
        Some("header names another region")
    );
    assert_eq!(
        decode((6, 126, 124)).err().as_deref(),
        Some("header names another region")
    );
    assert_ne!(
        decode((6, 126, 125)).err().as_deref(),
        Some("header names another region")
    );
}

#[test]
fn the_pinned_manifest_digest_matches_the_repository() -> Result<(), String> {
    let mut bytes = std::fs::read(repo().join(MANIFEST_PATH)).map_err(|e| e.to_string())?;
    bytes.retain(|byte| *byte != b'\r');
    let hex: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(hex, MANIFEST_SHA256);
    Ok(())
}

#[test]
fn a_crlf_checkout_of_the_manifest_still_matches_the_pin() -> Result<(), String> {
    let bytes = std::fs::read(repo().join(MANIFEST_PATH)).map_err(|e| e.to_string())?;
    let crlf: Vec<u8> = bytes
        .iter()
        .flat_map(|byte| match byte {
            b'\n' => vec![b'\r', b'\n'],
            other => vec![*other],
        })
        .collect();
    let dir = std::env::temp_dir().join(format!("oteryn-manifest-crlf-{}", std::process::id()));
    let manifest = dir.join("manifest.json");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(&manifest, crlf).map_err(|e| e.to_string())?;
    let result = AssetStore::open_pinned(
        &repo().join("content/assets/files"),
        &manifest,
        MANIFEST_SHA256,
    );
    std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    result.map_err(|e| e.to_string())?;
    Ok(())
}
