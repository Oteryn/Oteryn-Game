use oteryn_client::play::{PreferenceStore, ReferencePreferences};
#[test]
fn connection_optimisation_survives_the_admission_store_read()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("connection-preference.json");
    let preferences = ReferencePreferences {
        optimise_connection: true,
        ..ReferencePreferences::default()
    };
    PreferenceStore::export_to(&preferences, &path)?;
    let loaded = PreferenceStore::load(path.clone())?;
    assert!(loaded.preferences().optimise_connection);
    std::fs::remove_file(path)?;
    assert!(!ReferencePreferences::default().optimise_connection);
    Ok(())
}
