//! Local preferences. No identity, credentials or server authority are persisted here.
use crate::input::StepDir;
use oteryn_input_actions::{ButtonState, KeyCode, NormalizedInputEvent};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientSettings {
    pub version: u8,
    pub english: bool,
    pub fullscreen: bool,
    pub window_width: u16,
    pub window_height: u16,
    pub vsync: bool,
    /// Zero means no application frame limit.
    pub fps: u16,
    pub background_fps: u16,
    pub ui_scale: f32,
    pub high_contrast: bool,
    pub reduced_motion: bool,
    /// North, east, south, west; stable physical key codes.
    pub movement_keys: [u16; 4],
    pub click_to_walk: bool,
    #[serde(default = "enabled")]
    pub show_inventory: bool,
    #[serde(default = "enabled")]
    pub show_battle: bool,
    #[serde(default = "enabled")]
    pub show_chat: bool,
    #[serde(default = "enabled")]
    pub show_minimap: bool,
    #[serde(default)]
    pub action_bar: crate::action_bar::ActionBarPreferences,
    #[serde(default = "default_panel_shortcuts")]
    pub panel_shortcuts: Vec<String>,
    #[serde(default)]
    pub future_preferences:
        std::collections::BTreeMap<String, crate::settings_catalog::FutureValue>,
}

const fn enabled() -> bool {
    true
}

fn default_panel_shortcuts() -> Vec<String> {
    crate::panel_catalog::ShortcutOrder::default().saved_ids()
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            version: 1,
            english: false,
            fullscreen: false,
            window_width: 900,
            window_height: 620,
            vsync: true,
            fps: 120,
            background_fps: 30,
            ui_scale: 1.0,
            high_contrast: false,
            reduced_motion: false,
            movement_keys: [82, 79, 81, 80],
            click_to_walk: true,
            show_inventory: true,
            show_battle: true,
            show_chat: true,
            show_minimap: true,
            action_bar: crate::action_bar::ActionBarPreferences::default(),
            panel_shortcuts: default_panel_shortcuts(),
            future_preferences: std::collections::BTreeMap::new(),
        }
    }
}

impl ClientSettings {
    pub fn validate(&self) -> io::Result<()> {
        let keys_valid = self
            .movement_keys
            .iter()
            .all(|key| matches!(key, 4..=29 | 79..=82))
            && self
                .movement_keys
                .iter()
                .enumerate()
                .all(|(i, key)| !self.movement_keys[..i].contains(key));
        if self.version != 1
            || !(800..=3840).contains(&self.window_width)
            || !(600..=2160).contains(&self.window_height)
            || !(self.fps == 0 || (30..=360).contains(&self.fps))
            || !(5..=60).contains(&self.background_fps)
            || !self.ui_scale.is_finite()
            || !(0.8..=1.8).contains(&self.ui_scale)
            || !keys_valid
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid client preferences",
            ));
        }
        self.action_bar.validate(&self.movement_keys)?;
        crate::panel_catalog::ShortcutOrder::from_saved(&self.panel_shortcuts)?;
        crate::settings_catalog::validate_future_preferences(&self.future_preferences)
    }

    pub fn path() -> Option<PathBuf> {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|base| base.join("Oteryn").join("settings.json"))
            .or_else(|| {
                std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .map(|base| base.join("oteryn").join("settings.json"))
            })
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        if fs::metadata(path)?.len() > 32768 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Preferences too large",
            ));
        }
        let settings: Self = serde_json::from_slice(&fs::read(path)?)?;
        settings.validate()?;
        Ok(settings)
    }

    /// The old file remains intact if serialization or the temporary write fails.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        if bytes.len() > 32768 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Client preferences exceed their size bound",
            ));
        }
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&temporary, bytes)?;
        let file = fs::OpenOptions::new().write(true).open(&temporary)?;
        file.sync_all()?;
        // Rust's rename replaces an existing regular destination on Windows and Unix.
        fs::rename(temporary, path)
    }

    pub fn direction(&self, events: &[NormalizedInputEvent]) -> Option<StepDir> {
        events.iter().find_map(|event| {
            let NormalizedInputEvent::Key {
                code,
                state: ButtonState::Pressed,
                modifiers: oteryn_input_actions::Modifiers::NONE,
                ..
            } = event
            else {
                return None;
            };
            let index = self
                .movement_keys
                .iter()
                .position(|key| KeyCode::new(*key).ok() == Some(*code))?;
            [StepDir::North, StepDir::East, StepDir::South, StepDir::West]
                .get(index)
                .copied()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_preferences_and_duplicate_bindings_are_rejected() {
        let mut settings = ClientSettings::default();
        settings.movement_keys[1] = settings.movement_keys[0];
        assert!(settings.validate().is_err());
        settings = ClientSettings::default();
        settings.ui_scale = f32::NAN;
        assert!(settings.validate().is_err());
        settings = ClientSettings::default();
        settings.version = 2;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn modified_action_shortcut_does_not_also_walk() -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_input_actions::{Modifier, Modifiers};
        let settings = ClientSettings {
            movement_keys: [26, 7, 22, 4],
            ..ClientSettings::default()
        };
        let mut event = NormalizedInputEvent::Key {
            code: KeyCode::new(26)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control),
            repeat: false,
        };
        assert_eq!(settings.direction(&[event.clone()]), None);
        if let NormalizedInputEvent::Key { modifiers, .. } = &mut event {
            *modifiers = Modifiers::NONE;
        }
        assert_eq!(settings.direction(&[event]), Some(StepDir::North));
        Ok(())
    }

    #[test]
    fn preferences_survive_replacement_and_invalid_save_preserves_them() -> io::Result<()> {
        let path =
            std::env::temp_dir().join(format!("oteryn-settings-test-{}.json", std::process::id()));
        let mut settings = ClientSettings::default();
        settings.save(&path)?;
        settings.movement_keys = [26, 7, 22, 4];
        settings.english = true;
        settings.save(&path)?;
        assert_eq!(ClientSettings::load(&path)?, settings);
        settings.fps = 1;
        assert!(settings.save(&path).is_err());
        assert_eq!(ClientSettings::load(&path)?.fps, 120);
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn legacy_shortcuts_default_while_explicit_empty_and_custom_order_survive()
    -> Result<(), Box<dyn std::error::Error>> {
        let defaults = ClientSettings::default();
        let mut legacy = serde_json::to_value(&defaults)?;
        let object = legacy
            .as_object_mut()
            .ok_or("settings must serialize as an object")?;
        object.remove("panel_shortcuts");
        let restored: ClientSettings = serde_json::from_value(legacy.clone())?;
        restored.validate()?;
        assert_eq!(restored.panel_shortcuts, defaults.panel_shortcuts);
        legacy
            .as_object_mut()
            .ok_or("settings must serialize as an object")?
            .insert("panel_shortcuts".into(), serde_json::json!([]));
        let empty: ClientSettings = serde_json::from_value(legacy)?;
        empty.validate()?;
        assert!(empty.panel_shortcuts.is_empty());
        let custom = ClientSettings {
            english: true,
            panel_shortcuts: vec!["forge".into(), "skills".into(), "vip".into()],
            ..defaults
        };
        let loaded: ClientSettings = serde_json::from_slice(&serde_json::to_vec(&custom)?)?;
        loaded.validate()?;
        assert_eq!(loaded, custom);
        Ok(())
    }

    #[test]
    fn all_shortcuts_save_in_user_order_and_invalid_order_preserves_file() -> io::Result<()> {
        use crate::panel_catalog::ShortcutOrder;
        let path = std::env::temp_dir().join(format!(
            "oteryn-shortcut-order-test-{}.json",
            std::process::id()
        ));
        let mut settings = ClientSettings {
            english: true,
            panel_shortcuts: ShortcutOrder::all().saved_ids(),
            ..ClientSettings::default()
        };
        settings.panel_shortcuts.reverse();
        settings.save(&path)?;
        assert_eq!(ClientSettings::load(&path)?, settings);
        for invalid in [
            vec!["skills".into(), "skills".into()],
            vec!["unknown".into()],
            vec!["skills".into(); 29],
        ] {
            let mut rejected = settings.clone();
            rejected.panel_shortcuts = invalid;
            assert!(rejected.save(&path).is_err());
            assert_eq!(ClientSettings::load(&path)?, settings);
        }
        settings.panel_shortcuts = ShortcutOrder::default().saved_ids();
        settings.save(&path)?;
        let restored = ClientSettings::load(&path)?;
        assert_eq!(restored.panel_shortcuts.len(), 11);
        assert!(restored.english);
        fs::remove_file(path)?;
        Ok(())
    }
}
