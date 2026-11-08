#[test]
fn settings_transfer_backup_and_defaults_survive_restart() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::{PreferenceStore, ReferencePreferences, SettingsState};
    let root = std::env::temp_dir().join(format!("clientgap-transfer-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let path = root.join("settings.json");
    let export = root.join("export.json");
    let mut state = SettingsState::new(Some(path.clone()));
    let mut changed = state.preferences.clone();
    changed.ui_scale = 125;
    changed.audio.master_volume = 37;
    state.apply(changed.clone());
    state.export_to(&export)?;
    assert!(state.export_to(&export).is_err()); // Never overwrite an existing export.
    state.reset_defaults()?;
    assert_eq!(state.preferences, ReferencePreferences::default());
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences(),
        &state.preferences
    );
    state.restore_backup()?;
    assert_eq!(state.preferences, changed);
    state.reset_defaults()?;
    state.import_from(&export)?;
    assert_eq!(SettingsState::new(Some(path.clone())).preferences, changed);
    std::fs::write(root.join("bad.json"), br#"{"ui_scale":181}"#)?;
    let before = std::fs::read(&path)?;
    assert!(state.import_from(&root.join("bad.json")).is_err());
    assert_eq!(state.preferences, changed);
    assert_eq!(std::fs::read(&path)?, before);
    std::fs::write(root.join("huge.json"), vec![b' '; (1 << 20) + 1])?;
    assert!(state.import_from(&root.join("huge.json")).is_err());
    // Backup failure must prevent a destructive replacement.
    std::fs::remove_file(path.with_extension("backup.json"))?;
    std::fs::create_dir(path.with_extension("backup.json"))?;
    assert!(state.reset_defaults().is_err());
    assert_eq!(state.preferences, changed);
    assert_eq!(std::fs::read(&path)?, before);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
