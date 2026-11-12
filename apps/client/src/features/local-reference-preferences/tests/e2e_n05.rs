//! N05 (client-local, no server): export, import into a fresh store, backup, defaults and a corrupt
//! import, driven through the production `SettingsState` / `PreferenceStore` path and checked
//! against the exact JSON the files hold.
use crate::play::{PreferenceStore, ReferencePreferences, SettingsState};
use serde_json::{Value, json};

type Error = Box<dyn std::error::Error>;
const HIGHSCORES: &str = "0009-highscores-open";

fn defaults_json() -> Value {
    json!({
        "audio": {"master_volume": 100, "muted": false},
        "keybindings": {},
        "english": false,
        "ui_scale": 100,
        "visible_panels": ["battle", "chat", "inventory", "minimap"],
    })
}

fn changed_json() -> Value {
    json!({
        "audio": {"master_volume": 37, "muted": true},
        "keybindings": {HIGHSCORES: "ALT+H"},
        "english": true,
        "ui_scale": 125,
        "visible_panels": ["battle", "wheel"],
    })
}

fn file(path: &std::path::Path) -> Result<Value, Error> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

/// Every field of `picked` (recursively) has the same value in `actual`.
fn contains(actual: &Value, picked: &Value) -> bool {
    match (actual, picked) {
        (Value::Object(actual), Value::Object(picked)) => picked
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|held| contains(held, value))),
        _ => actual == picked,
    }
}

/// The file holds exactly `preferences`, and the fields this journey changes have the values
/// written out in `picked`.
fn holds(path: &std::path::Path, preferences: &ReferencePreferences, picked: &Value) -> Result<(), Error> {
    let actual = file(path)?;
    assert_eq!(actual, serde_json::to_value(preferences)?);
    assert!(contains(&actual, picked), "{} lacks {picked}", path.display());
    Ok(())
}

#[test]
fn e2e_n05_settings_export_import_backup_defaults_and_corrupt_file() -> Result<(), Error> {
    let root = std::env::temp_dir().join(format!("oteryn-n05-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    let (active, export, fresh) = (
        root.join("settings.json"),
        root.join("export.json"),
        root.join("fresh.json"),
    );

    // Change every kind of preference and save it through the production apply path.
    let mut state = SettingsState::new(Some(active.clone()));
    assert_eq!(state.preferences, ReferencePreferences::default());
    let mut changed = state.preferences.clone();
    changed.rebind(HIGHSCORES, "Alt+H")?;
    changed.ui_scale = 125;
    changed.audio.master_volume = 37;
    changed.audio.muted = true;
    changed.english = true;
    changed.visible_panels = vec!["battle".into(), "wheel".into()];
    state.apply(changed.clone());
    assert_eq!(state.feedback, "Applied and saved");
    holds(&active, &changed, &changed_json())?;

    // Export never overwrites an existing file.
    state.export_to(&export)?;
    holds(&export, &changed, &changed_json())?;
    assert!(state.export_to(&export).is_err());
    holds(&export, &changed, &changed_json())?;

    // A fresh store starts at the defaults, imports the export, and keeps its previous values as
    // the backup.
    let mut other = SettingsState::new(Some(fresh.clone()));
    assert_eq!(other.preferences, ReferencePreferences::default());
    other.import_from(&export)?;
    assert_eq!(other.preferences, changed);
    holds(&fresh, &changed, &changed_json())?;
    let backup = fresh.with_extension("backup.json");
    holds(&backup, &ReferencePreferences::default(), &defaults_json())?;
    assert_eq!(PreferenceStore::load(fresh.clone())?.preferences(), &changed);

    // Defaults replace the live values and move the imported ones into the backup, which restores.
    other.reset_defaults()?;
    assert_eq!(other.preferences, ReferencePreferences::default());
    holds(&fresh, &ReferencePreferences::default(), &defaults_json())?;
    holds(&backup, &changed, &changed_json())?;
    other.restore_backup()?;
    assert_eq!(other.preferences, changed);
    holds(&fresh, &changed, &changed_json())?;

    // Corrupt, out-of-range and unknown-field files are refused and change nothing.
    let (stored, stored_backup) = (std::fs::read(&fresh)?, std::fs::read(&backup)?);
    for (name, bytes) in [
        ("truncated.json", br#"{"ui_scale":1"#.to_vec()),
        ("range.json", br#"{"ui_scale":181}"#.to_vec()),
        ("unknown.json", br#"{"ui_scale":100,"surprise":true}"#.to_vec()),
        ("panel.json", br#"{"visible_panels":["unregistered-panel"]}"#.to_vec()),
    ] {
        let bad = root.join(name);
        std::fs::write(&bad, bytes)?;
        assert!(other.import_from(&bad).is_err(), "{name} was accepted");
        assert_eq!(other.preferences, changed, "{name} changed the settings");
        assert_eq!(std::fs::read(&fresh)?, stored, "{name} rewrote the store");
        assert_eq!(std::fs::read(&backup)?, stored_backup, "{name} rewrote the backup");
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}
