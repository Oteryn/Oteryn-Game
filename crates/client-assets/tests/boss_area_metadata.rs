use oteryn_client_assets::AssetStore;
#[test]
fn pinned_manifest_owns_boss_area_classification() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("oteryn-boss-effects-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("manifest.json");
    let bytes = br#"{"files":[],"boss_area_effects":[41]}"#;
    std::fs::write(&path, bytes)?;
    let store = AssetStore::open_pinned(&dir, &path, &oteryn_client_assets::text_sha256(bytes))?;
    assert!(store.effects().is_boss_area(41));
    assert!(!store.effects().is_boss_area(42));
    std::fs::write(&path, br#"{"files":[],"boss_area_effects":[42]}"#)?;
    assert!(
        AssetStore::open_pinned(&dir, &path, &oteryn_client_assets::text_sha256(bytes)).is_err()
    );
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn boss_area_metadata_rejects_invalid_and_duplicate_ids() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = std::env::temp_dir().join(format!("oteryn-boss-invalid-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("manifest.json");
    for ids in [vec![0], vec![65536], vec![41, 41]] {
        let bytes =
            serde_json::to_vec(&serde_json::json!({ "files": [], "boss_area_effects": ids }))?;
        std::fs::write(&path, &bytes)?;
        assert!(
            AssetStore::open_pinned(&dir, &path, &oteryn_client_assets::text_sha256(&bytes))
                .is_err()
        );
    }
    std::fs::remove_dir_all(dir)?;
    Ok(())
}
