#[test]
fn every_registered_binding_conflicts_and_invalid_saves_leave_preferences_unchanged()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PreferenceStore, ReferencePreferences, preference_bindings};
    let original = ReferencePreferences::default();
    for binding in preference_bindings()? {
        if binding.id == "0009-highscores-open" {
            continue;
        }
        let mut preferences = original.clone();
        assert!(
            preferences
                .rebind("0009-highscores-open", binding.default_chord)
                .is_err(),
            "{}",
            binding.id
        );
        assert_eq!(preferences, original);
    }
    let mut preferences = original.clone();
    assert!(preferences.rebind("unknown", "ALT+H").is_err());
    assert!(
        preferences
            .rebind("0009-highscores-open", "CTRL+CTRL+H")
            .is_err()
    );
    let path = std::env::temp_dir().join(format!("oteryn-c25-invalid-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut store = PreferenceStore::load(path.clone())?;
    preferences.ui_scale = 181;
    assert!(store.apply(preferences).is_err());
    assert_eq!(store.preferences(), &original);
    assert!(!path.exists());
    std::fs::write(&path, b"invalid-json")?;
    assert!(PreferenceStore::load(path.clone()).is_err());
    assert_eq!(std::fs::read(&path)?, b"invalid-json");
    std::fs::remove_file(&path)?;
    std::fs::create_dir(&path)?;
    let mut blocked = PreferenceStore::defaults(path.clone());
    let mut changed = original.clone();
    changed.audio.master_volume = 35;
    assert!(blocked.apply(changed).is_err());
    assert_eq!(blocked.preferences(), &original);
    assert!(!path.with_extension("json.tmp").exists());
    std::fs::remove_dir(path)?;
    Ok(())
}
