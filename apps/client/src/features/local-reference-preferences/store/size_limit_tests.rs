#[cfg(test)]
mod size_limit_tests {
    use super::*;
    #[test]
    fn oversized_writes_preserve_existing_stores() -> Result<(), Box<dyn std::error::Error>> {
        let fixture = crate::play::screenshot::TestDir::new()?;
        let root = fixture.path().to_path_buf();
        let path = root.join("settings.json");
        let mut store = PreferenceStore::defaults(path.clone());
        store.apply(ReferencePreferences::default())?;
        let original = std::fs::read(&path)?;
        let mut oversized = ReferencePreferences::default();
        oversized.custom_hotkeys.clear();
        for modifier in ["SUPER+", "CTRL+SUPER+", "ALT+SUPER+"] {
            for key in 'A'..='Z' {
                if oversized.custom_hotkeys.len() == 64 {
                    break;
                }
                let chord = format!("{modifier}{key}");
                let _ = oversized.assign_custom_hotkey(
                    &chord,
                    super::super::CustomAction::Text {
                        text: "x".repeat(1020),
                        send: true,
                    },
                );
            }
        }
        for index in 0..16 {
            oversized.save_hotkey_profile(&format!("Snapshot {index}"))?;
        }
        oversized.validate()?;
        assert!(serde_json::to_vec_pretty(&oversized)?.len() > 1 << 20);
        assert!(store.apply(oversized.clone()).is_err());
        assert_eq!(std::fs::read(&path)?, original);
        assert_eq!(store.preferences(), &ReferencePreferences::default());
        let export = root.join("export.json");
        assert!(PreferenceStore::export_to(&oversized, &export).is_err());
        assert!(!export.exists());
        std::fs::write(store.backup_path(), b"previous backup")?;
        assert!(
            store
                .replace_with_backup(oversized.clone(), &ReferencePreferences::default())
                .is_err()
        );
        assert_eq!(std::fs::read(store.backup_path())?, b"previous backup");
        assert!(
            store
                .replace_with_backup(ReferencePreferences::default(), &oversized)
                .is_err()
        );
        assert_eq!(std::fs::read(&path)?, original);
        std::fs::write(&path, vec![b' '; (1 << 20) + 1])?;
        let rejected = std::fs::read(&path)?;
        let mut recovered = super::super::SettingsState::new(Some(path.clone()));
        recovered.apply(ReferencePreferences::default());
        assert_eq!(std::fs::read(&path)?, rejected);
        let backup_preferences = ReferencePreferences {
            ui_scale: 130,
            ..Default::default()
        };
        let backup_bytes = serde_json::to_vec_pretty(&backup_preferences)?;
        std::fs::write(store.backup_path(), &backup_bytes)?;
        assert!(
            store
                .replace_with_backup(
                    ReferencePreferences::default(),
                    &ReferencePreferences::default()
                )
                .is_err()
        );
        assert_eq!(std::fs::read(store.backup_path())?, backup_bytes);
        assert_eq!(std::fs::read(&path)?, rejected);
        Ok(())
    }
}
