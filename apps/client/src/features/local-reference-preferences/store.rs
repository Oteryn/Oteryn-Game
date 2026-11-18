//! Validated, atomic replacement of the native client's local preferences.
use super::ReferencePreferences;
use std::{
    io,
    path::{Path, PathBuf},
};

pub struct PreferenceStore {
    path: PathBuf,
    preferences: ReferencePreferences,
}

impl PreferenceStore {
    pub fn character_path(base: &Path, identity: [u8; 16]) -> PathBuf {
        let name: String = identity.iter().map(|byte| format!("{byte:02x}")).collect();
        base.with_extension("characters")
            .join(format!("{name}.json"))
    }

    /// Read a portable settings file with the same validation as local storage.
    pub fn read_transfer(path: &Path) -> io::Result<ReferencePreferences> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take((1 << 20) + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 1 << 20 {
            return Err(io::Error::other(tr!("Settings file exceeds 1 MiB")));
        }
        let mut document: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        if let Some(fields) = document.as_object_mut()
            && fields.get("hotkey_profiles").is_some_and(|value| {
                value
                    .get("presets")
                    .is_some_and(serde_json::Value::is_object)
                    && value
                        .get("selected")
                        .is_some_and(serde_json::Value::is_string)
                    && value
                        .get("chat_on")
                        .is_some_and(serde_json::Value::is_boolean)
            })
        {
            if fields.contains_key("hotkey_contexts") {
                return Err(io::Error::other("Conflicting hotkey context formats"));
            }
            if let Some(contexts) = fields.remove("hotkey_profiles") {
                fields.insert("hotkey_contexts".into(), contexts);
            }
        }
        let legacy = document
            .as_object()
            .is_some_and(|fields| !fields.contains_key("custom_hotkeys"));
        if legacy {
            document["custom_hotkeys"] = serde_json::json!({});
        }
        let mut preferences: ReferencePreferences =
            serde_json::from_value(document).map_err(io::Error::other)?;
        if legacy {
            for (chord, action) in super::custom_actions::default_custom_hotkeys() {
                let _ = preferences.assign_custom_hotkey(&chord, action);
            }
        }
        preferences.upgrade_schema();
        preferences.validate().map_err(io::Error::other)?;
        preferences.reconcile_controls(None);
        Ok(preferences)
    }

    /// Exports never replace existing files, including the active preferences.
    pub fn export_to(preferences: &ReferencePreferences, path: &Path) -> io::Result<()> {
        use std::io::Write;
        let bytes = Self::serialized(preferences)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
            let _ = std::fs::remove_file(path);
            return Err(error);
        }
        Ok(())
    }

    fn serialized(preferences: &ReferencePreferences) -> io::Result<Vec<u8>> {
        preferences.validate().map_err(io::Error::other)?;
        let bytes = serde_json::to_vec_pretty(preferences).map_err(io::Error::other)?;
        if bytes.len() > 1 << 20 {
            return Err(io::Error::other(tr!("Settings file exceeds 1 MiB")));
        }
        Ok(bytes)
    }

    pub fn backup_path(&self) -> PathBuf {
        self.path.with_extension("backup.json")
    }

    pub fn replace_with_backup(
        &mut self,
        preferences: ReferencePreferences,
        previous: &ReferencePreferences,
    ) -> io::Result<()> {
        Self::serialized(&preferences)?;
        Self::serialized(previous)?;
        if self.path.try_exists()? {
            Self::read_transfer(&self.path)?;
        }
        let mut backup = Self::defaults(self.backup_path());
        backup.apply(previous.clone())?;
        self.apply(preferences)
    }

    pub fn defaults(path: PathBuf) -> Self {
        Self {
            path,
            preferences: ReferencePreferences::default(),
        }
    }

    pub fn load(path: PathBuf) -> io::Result<Self> {
        let preferences = match Self::read_transfer(&path) {
            Ok(preferences) => preferences,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                ReferencePreferences::default()
            }
            Err(error) => return Err(error),
        };
        Ok(Self { path, preferences })
    }

    pub fn preferences(&self) -> &ReferencePreferences {
        &self.preferences
    }

    pub fn apply(&mut self, preferences: ReferencePreferences) -> io::Result<()> {
        let bytes = Self::serialized(&preferences)?;
        if self.path.try_exists()? {
            Self::read_transfer(&self.path)?;
        }
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = self.path.with_extension("json.tmp");
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }
        if let Err(error) = std::fs::rename(&temporary, &self.path) {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
        self.preferences = preferences;
        Ok(())
    }

    pub fn path() -> Option<PathBuf> {
        Self::configuration_path("client-preferences.json")
    }

    /// Shared native configuration location; explicit host paths bypass this default.
    pub(crate) fn configuration_path(filename: &str) -> Option<PathBuf> {
        let platform = if cfg!(windows) {
            ConfigurationPlatform::Windows
        } else if cfg!(target_os = "macos") {
            ConfigurationPlatform::MacOs
        } else {
            ConfigurationPlatform::Unix
        };
        configuration_path_for(
            platform,
            filename,
            |name| std::env::var_os(name),
            |path| {
                // Keep an inaccessible file selected so the loader reports its error.
                path.try_exists().unwrap_or(true)
            },
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConfigurationPlatform {
    Windows,
    MacOs,
    Unix,
}

fn configuration_path_for(
    platform: ConfigurationPlatform,
    filename: &str,
    lookup: impl Fn(&str) -> Option<std::ffi::OsString>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let home = || lookup("HOME").map(PathBuf::from);
    let unix_root = || {
        lookup("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home().map(|root| root.join(".config")))
    };
    let root = match platform {
        ConfigurationPlatform::Windows => lookup("LOCALAPPDATA").map(PathBuf::from),
        ConfigurationPlatform::MacOs => home().map(|root| root.join("Library/Application Support")),
        ConfigurationPlatform::Unix => unix_root(),
    };
    let preferred = root.map(|root| root.join("Oteryn").join(filename));
    // Existing macOS clients used the Unix path. Prefer the native file once one exists,
    // otherwise retain an existing legacy file without copying or changing its schema.
    if platform == ConfigurationPlatform::MacOs
        && preferred.as_ref().is_none_or(|path| !exists(path))
        && let Some(legacy) = unix_root()
            .map(|root| root.join("Oteryn").join(filename))
            .filter(|path| exists(path))
    {
        return Some(legacy);
    }
    preferred
}

#[cfg(test)]
mod merge_compatibility {
    use super::*;
    #[test]
    fn rich_options_profiles_load_after_context_extraction()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = crate::play::screenshot::TestDir::new()?;
        let path = fixture.join("clientgap-profile-merge.json");
        let mut preferences = ReferencePreferences::default();
        preferences.rebind("0009-highscores-open", "ALT+H")?;
        preferences.add_hotkey_profile("Merge compatibility")?;
        let mut old = serde_json::to_value(&preferences)?;
        let fields = old.as_object_mut().ok_or("preference object")?;
        let contexts = fields.remove("hotkey_contexts").ok_or("contexts")?;
        fields.insert("hotkey_profiles".into(), contexts);
        std::fs::write(&path, serde_json::to_vec(&old)?)?;
        assert_eq!(PreferenceStore::read_transfer(&path)?, preferences);
        std::fs::remove_file(path)?;
        Ok(())
    }
}

#[cfg(test)]
mod legacy_chords {
    use super::*;
    #[test]
    fn legacy_defaults_preserve_primary_secondary_and_action_bar_chords()
    -> Result<(), Box<dyn std::error::Error>> {
        for (field, value, chord) in [
            (
                "keybindings",
                serde_json::json!({"0009-highscores-open":"ALT+H", "local-spell-action-bar-spell-1":"CTRL+H"}),
                "CTRL+H",
            ),
            (
                "secondary_keybindings",
                serde_json::json!({"0009-highscores-open":"CTRL+T"}),
                "CTRL+T",
            ),
            ("action_keys", serde_json::json!({"0":"CTRL+E"}), "CTRL+E"),
        ] {
            let fixture = crate::play::screenshot::TestDir::new()?;
            let path = fixture.join(format!("legacy-chords-{field}.json"));
            let mut doc = serde_json::to_value(ReferencePreferences::default())?;
            doc.as_object_mut()
                .ok_or("object")?
                .remove("custom_hotkeys");
            doc[field] = value.clone();
            std::fs::write(&path, serde_json::to_vec(&doc)?)?;
            let loaded = PreferenceStore::read_transfer(&path);
            std::fs::remove_file(&path)?;
            let loaded = loaded?;
            assert!(!loaded.custom_hotkeys.contains_key(chord));
            assert_eq!(serde_json::to_value(&loaded)?[field], value);
            assert_eq!(loaded.custom_hotkeys.len(), 11);
        }
        Ok(())
    }

    /// A profile saved before the Global Prey/VIP/skills/options/server-messages/manual-sort
    /// defaults gains each of those chords once, unless the player already uses the chord.
    #[test]
    fn saved_custom_profiles_gain_unused_global_chords_once_without_overriding_the_player()
    -> Result<(), Box<dyn std::error::Error>> {
        use super::super::{CustomAction, UtilityAction};
        let text = |text: &str| CustomAction::Text {
            text: text.into(),
            send: false,
        };
        let utility = |action| CustomAction::Utility { action };
        let fixture = crate::play::screenshot::TestDir::new()?;
        let path = fixture.join("global-chords-upgrade.json");
        let mut doc = serde_json::to_value(ReferencePreferences::default())?;
        let fields = doc.as_object_mut().ok_or("object")?;
        fields.remove("schema_version");
        fields.insert(
            "custom_hotkeys".into(),
            serde_json::json!({
                "CTRL+Y": {"kind": "text", "text": "hi", "send": false},
                "F5": {"kind": "utility", "action": "show_prey"},
            }),
        );
        fields.insert(
            "keybindings".into(),
            serde_json::json!({"0009-highscores-open": "CTRL+K"}),
        );
        // Paladin was saved without a custom map; its own bindings hold Ctrl+K.
        fields.insert(
            "hotkey_profiles".into(),
            serde_json::json!({"Knight": {}, "Paladin": {"0009-highscores-open": "CTRL+K"}}),
        );
        fields.insert(
            "custom_hotkey_profiles".into(),
            serde_json::json!({"Knight": {"CTRL+P": {"kind": "text", "text": "exura", "send": true}}}),
        );
        std::fs::write(&path, serde_json::to_vec(&doc)?)?;
        let mut store = PreferenceStore::load(path.clone());
        let loaded = store
            .as_ref()
            .map(|store| store.preferences().clone())
            .map_err(|error| error.to_string());
        let upgraded = loaded.clone()?;
        let hotkeys = &upgraded.custom_hotkeys;
        assert_eq!(hotkeys.get("CTRL+Y"), Some(&text("hi")));
        assert_eq!(hotkeys.get("F5"), Some(&utility(UtilityAction::ShowPrey)));
        assert!(!hotkeys.contains_key("CTRL+K"));
        for (chord, action) in [
            ("CTRL+M", UtilityAction::ToggleServerMessages),
            ("CTRL+P", UtilityAction::ToggleVipWidget),
            ("ALT+S", UtilityAction::ToggleSkillsWidget),
            ("SHIFT+S", UtilityAction::ToggleManualSortMode),
        ] {
            assert_eq!(hotkeys.get(chord), Some(&utility(action)), "{chord}");
        }
        assert_eq!(hotkeys.len(), 6);
        let knight = upgraded
            .custom_hotkey_profiles
            .get("Knight")
            .ok_or("Knight profile")?;
        assert_eq!(
            knight.get("CTRL+P"),
            Some(&CustomAction::Text {
                text: "exura".into(),
                send: true
            })
        );
        for (chord, action) in [
            ("CTRL+M", UtilityAction::ToggleServerMessages),
            ("CTRL+Y", UtilityAction::ShowPrey),
            ("ALT+S", UtilityAction::ToggleSkillsWidget),
            ("CTRL+K", UtilityAction::ShowOptionsHotkeys),
            ("SHIFT+S", UtilityAction::ToggleManualSortMode),
        ] {
            assert_eq!(knight.get(chord), Some(&utility(action)), "Knight {chord}");
        }
        let mut paladin = upgraded.clone();
        paladin.load_hotkey_profile("Paladin")?;
        for (chord, action) in [
            ("CTRL+M", UtilityAction::ToggleServerMessages),
            ("CTRL+Y", UtilityAction::ShowPrey),
            ("CTRL+P", UtilityAction::ToggleVipWidget),
            ("ALT+S", UtilityAction::ToggleSkillsWidget),
            ("SHIFT+S", UtilityAction::ToggleManualSortMode),
        ] {
            assert_eq!(
                paladin.custom_hotkeys.get(chord),
                Some(&utility(action)),
                "Paladin {chord}"
            );
        }
        assert_eq!(paladin.custom_hotkeys.len(), 5, "Ctrl+K stays the profile's");
        // The upgrade runs once: a chord the player removes afterwards stays removed.
        let mut edited = upgraded;
        edited.custom_hotkeys.remove("CTRL+P");
        let saved = store.as_mut().map(|store| store.apply(edited));
        let reloaded = PreferenceStore::load(path.clone()).map(|store| store.preferences().clone());
        std::fs::remove_file(&path)?;
        saved.map_err(|error| error.to_string())??;
        let reloaded = reloaded?;
        assert!(!reloaded.custom_hotkeys.contains_key("CTRL+P"));
        assert_eq!(reloaded.custom_hotkeys.len(), 5);
        Ok(())
    }
}

#[cfg(test)]
mod native_configuration_path_tests {
    use super::*;
    use std::ffi::OsString;

    fn lookup(name: &str) -> Option<OsString> {
        match name {
            "LOCALAPPDATA" => Some("/windows/local".into()),
            "HOME" => Some("/users/player".into()),
            "XDG_CONFIG_HOME" => Some("/xdg/config".into()),
            _ => None,
        }
    }

    #[test]
    fn default_files_use_the_native_platform_configuration_directory() {
        for (platform, expected) in [
            (
                ConfigurationPlatform::Windows,
                "/windows/local/Oteryn/client-preferences.json",
            ),
            (
                ConfigurationPlatform::MacOs,
                "/users/player/Library/Application Support/Oteryn/client-preferences.json",
            ),
            (
                ConfigurationPlatform::Unix,
                "/xdg/config/Oteryn/client-preferences.json",
            ),
        ] {
            assert_eq!(
                configuration_path_for(platform, "client-preferences.json", lookup, |_| false),
                Some(PathBuf::from(expected))
            );
        }
        assert_eq!(
            configuration_path_for(
                ConfigurationPlatform::Unix,
                "client-preferences.json",
                |name| (name == "HOME").then(|| "/users/player".into()),
                |_| false
            ),
            Some(PathBuf::from(
                "/users/player/.config/Oteryn/client-preferences.json"
            ))
        );
        assert_eq!(
            configuration_path_for(
                ConfigurationPlatform::Unix,
                "client-preferences.json",
                |_| None,
                |_| false
            ),
            None
        );
    }

    #[test]
    fn existing_macos_login_file_is_retained_until_the_native_file_exists() {
        let legacy = PathBuf::from("/xdg/config/Oteryn/native-login.json");
        let native =
            PathBuf::from("/users/player/Library/Application Support/Oteryn/native-login.json");
        assert_eq!(
            configuration_path_for(
                ConfigurationPlatform::MacOs,
                "native-login.json",
                lookup,
                |path| path == legacy
            ),
            Some(legacy.clone())
        );
        assert_eq!(
            configuration_path_for(
                ConfigurationPlatform::MacOs,
                "native-login.json",
                lookup,
                |path| path == legacy || path == native
            ),
            Some(native)
        );
    }
}

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
