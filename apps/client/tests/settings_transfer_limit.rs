use oteryn_client::play::PreferenceStore;

#[test]
fn active_settings_use_transfer_size_limit() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("oteryn-settings-limit.json");
    let mut bytes = serde_json::to_vec(&oteryn_client::play::ReferencePreferences::default())?;
    bytes.resize((1 << 20) + 1, b' ');
    std::fs::write(&path, bytes)?;
    let result = PreferenceStore::load(path.clone());
    std::fs::remove_file(path)?;
    assert!(
        result.is_err(),
        "active settings must enforce the import size limit"
    );
    Ok(())
}
