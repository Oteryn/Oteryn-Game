//! Local preferences. No identity, credentials or server authority are persisted here.
use crate::input::StepDir;
use oteryn_input_actions::{ButtonState, KeyCode, NormalizedInputEvent};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Bound hostile or corrupt local input while leaving room for all 32 complete
/// hotkey profiles, their two bindings per action, and custom actions.
const MAX_SETTINGS_BYTES: u64 = 8 * 1024 * 1024;

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
    /// Background transparency of the settings window, in percent.
    #[serde(default = "default_settings_transparency")]
    pub settings_transparency: u8,
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
    #[serde(default)]
    pub hotkeys: crate::hotkeys::HotkeyPreferences,
    #[serde(default = "default_panel_shortcuts")]
    pub panel_shortcuts: Vec<String>,
    #[serde(default)]
    pub future_preferences:
        std::collections::BTreeMap<String, crate::settings_catalog::FutureValue>,
}

const fn default_settings_transparency() -> u8 {
    10
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
            settings_transparency: default_settings_transparency(),
            reduced_motion: false,
            movement_keys: [82, 79, 81, 80],
            click_to_walk: true,
            show_inventory: true,
            show_battle: true,
            show_chat: true,
            show_minimap: true,
            action_bar: crate::action_bar::ActionBarPreferences::default(),
            hotkeys: crate::hotkeys::HotkeyPreferences::default(),
            panel_shortcuts: default_panel_shortcuts(),
            future_preferences: std::collections::BTreeMap::new(),
        }
    }
}

impl ClientSettings {
    /// Local count control enables the first N rows of the chosen edge, preserving
    /// every row's locks and chords. The detailed editor can select arbitrary rows.
    pub fn set_action_row_count(&mut self, edge: usize, count: u8) -> io::Result<()> {
        if edge >= 3 || count > 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid action row count",
            ));
        }
        for offset in 0..3 {
            let index = edge * 3 + offset;
            let mut row = self
                .action_bar
                .row(index)
                .ok_or_else(|| io::Error::other("Missing action row"))?;
            row.visible = offset < usize::from(count);
            self.action_bar.set_row(index, row)?;
        }
        Ok(())
    }

    /// Previously inert count choices acquire their supported local consumer on load.
    /// Stored choices take precedence over visibility; bindings/locks remain untouched.
    fn migrate_legacy_action_rows(&mut self) -> io::Result<()> {
        for (edge, key) in [
            "action_bars.bars.bottom_rows",
            "action_bars.bars.left_rows",
            "action_bars.bars.right_rows",
        ]
        .iter()
        .enumerate()
        {
            if let Some(value) = self.future_preferences.remove(*key) {
                let crate::settings_catalog::FutureValue::Int(count @ 0..=3) = value else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid legacy action row count",
                    ));
                };
                self.set_action_row_count(edge, count as u8)?;
            }
        }
        Ok(())
    }

    /// Consume explicit former draft intent once, matching the count migration's
    /// precedence. Missing keys retain the canonical typed value and never invent
    /// a reference preference. Masters preserve each row's selection, lock and chord.
    fn migrate_legacy_action_edge_masters(&mut self) -> io::Result<()> {
        for (edge, key) in [
            "action_bars.bars.bottom_visible",
            "action_bars.bars.left_visible",
            "action_bars.bars.right_visible",
        ]
        .iter()
        .enumerate()
        {
            if let Some(value) = self.future_preferences.remove(*key) {
                let crate::settings_catalog::FutureValue::Bool(enabled) = value else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Invalid legacy action edge master",
                    ));
                };
                self.action_bar.edge_enabled[edge] = enabled;
            }
        }
        Ok(())
    }

    pub(crate) fn reconcile_active_hotkey_profile(&mut self) -> io::Result<()> {
        let active = self
            .hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing hotkey profile"))?;
        active.action_bar = self.action_bar.clone();
        Ok(())
    }

    pub fn select_hotkey_profile(&mut self, index: usize) -> io::Result<()> {
        if index >= self.hotkeys.profiles.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unknown hotkey profile",
            ));
        }
        self.hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing hotkey profile"))?
            .action_bar = self.action_bar.clone();
        self.hotkeys.selected = index;
        self.action_bar = self.hotkeys.profiles[index].action_bar.clone();
        Ok(())
    }

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
            || self.settings_transparency > 70
            || !keys_valid
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid client preferences",
            ));
        }
        self.action_bar.validate(&self.movement_keys)?;
        self.hotkeys.validate(&self.movement_keys)?;
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
        if fs::metadata(path)?.len() > MAX_SETTINGS_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Preferences too large",
            ));
        }
        let mut settings: Self = serde_json::from_slice(&fs::read(path)?)?;
        settings.migrate_legacy_action_rows()?;
        settings.migrate_legacy_action_edge_masters()?;
        settings.reconcile_active_hotkey_profile()?;
        settings.validate()?;
        Ok(settings)
    }

    /// The old file remains intact if serialization or the temporary write fails.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut persisted = self.clone();
        persisted.reconcile_active_hotkey_profile()?;
        let bytes = serde_json::to_vec(&persisted)?;
        if bytes.len() as u64 > MAX_SETTINGS_BYTES {
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
    fn settings_transparency_migrates_and_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let mut value = serde_json::to_value(ClientSettings::default())?;
        value
            .as_object_mut()
            .ok_or("settings object")?
            .remove("settings_transparency");
        let mut settings: ClientSettings = serde_json::from_value(value)?;
        assert_eq!(settings.settings_transparency, 10);
        for transparency in [0, 10, 70] {
            settings.settings_transparency = transparency;
            settings.validate()?;
            let restored: ClientSettings = serde_json::from_slice(&serde_json::to_vec(&settings)?)?;
            assert_eq!(restored, settings);
        }
        settings.settings_transparency = 71;
        assert!(settings.validate().is_err());
        Ok(())
    }

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

    #[test]
    fn legacy_future_row_counts_migrate_without_losing_chords_locks_or_other_choices()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::settings_catalog::FutureValue;
        let path = std::env::temp_dir().join(format!(
            "oteryn-legacy-action-rows-{}.json",
            std::process::id()
        ));
        let mut old = ClientSettings::default();
        old.action_bar.locked = true;
        old.future_preferences.extend([
            ("action_bars.bars.bottom_rows".into(), FutureValue::Int(2)),
            ("action_bars.bars.left_rows".into(), FutureValue::Int(1)),
            ("action_bars.bars.right_rows".into(), FutureValue::Int(3)),
            ("sound.sound.master".into(), FutureValue::Int(40)),
        ]);
        let mut legacy = serde_json::to_value(&old)?;
        legacy["action_bar"]
            .as_object_mut()
            .ok_or("action bar must be an object")?
            .remove("extra_rows");
        fs::write(&path, serde_json::to_vec(&legacy)?)?;
        let migrated = ClientSettings::load(&path)?;
        assert_eq!(migrated.action_bar.visible_rows(), [2, 1, 3]);
        assert_eq!(migrated.action_bar.shortcuts, old.action_bar.shortcuts);
        assert!(migrated.action_bar.locked);
        assert_eq!(migrated.future_preferences.len(), 1);
        assert_eq!(
            migrated.future_preferences["sound.sound.master"],
            FutureValue::Int(40)
        );
        migrated.save(&path)?;
        assert_eq!(ClientSettings::load(&path)?, migrated);
        // Invalid former intent fails closed rather than becoming a fabricated count.
        legacy["future_preferences"]["action_bars.bars.bottom_rows"] =
            serde_json::to_value(FutureValue::Int(4))?;
        fs::write(&path, serde_json::to_vec(&legacy)?)?;
        assert!(ClientSettings::load(&path).is_err());
        legacy["future_preferences"]["action_bars.bars.bottom_rows"] =
            serde_json::to_value(FutureValue::Bool(true))?;
        fs::write(&path, serde_json::to_vec(&legacy)?)?;
        assert!(ClientSettings::load(&path).is_err());
        legacy["future_preferences"]["action_bars.bars.bottom_rows"] =
            serde_json::to_value(FutureValue::Int(-1))?;
        fs::write(&path, serde_json::to_vec(&legacy)?)?;
        assert!(ClientSettings::load(&path).is_err());
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn zero_legacy_count_preserves_hidden_row_chords_and_other_edges() -> io::Result<()> {
        use crate::{action_bar::SlotShortcut, settings_catalog::FutureValue};
        let mut settings = ClientSettings::default();
        let mut left = settings
            .action_bar
            .row(4)
            .ok_or_else(|| io::Error::other("missing action row"))?;
        left.visible = true;
        left.locked = true;
        left.shortcuts[0] = Some(SlotShortcut {
            key: 4,
            modifiers: 2,
        });
        settings.action_bar.set_row(4, left)?;
        let mut right = settings
            .action_bar
            .row(8)
            .ok_or_else(|| io::Error::other("missing action row"))?;
        right.visible = true;
        settings.action_bar.set_row(8, right)?;
        settings
            .future_preferences
            .insert("action_bars.bars.left_rows".into(), FutureValue::Int(0));
        settings.migrate_legacy_action_rows()?;
        assert_eq!(settings.action_bar.visible_rows(), [1, 0, 1]);
        left.visible = false;
        assert_eq!(settings.action_bar.row(4), Some(left));
        assert_eq!(settings.action_bar.row(8), Some(right));
        assert!(settings.future_preferences.is_empty());
        settings.validate()?;
        let before = settings.clone();
        assert!(settings.set_action_row_count(3, 1).is_err());
        assert!(settings.set_action_row_count(0, 4).is_err());
        assert_eq!(settings, before);
        Ok(())
    }

    #[test]
    fn explicit_legacy_edge_masters_migrate_once_without_erasing_raw_rows()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::{action_bar::SlotShortcut, settings_catalog::FutureValue};
        let path = std::env::temp_dir().join(format!(
            "oteryn-legacy-edge-masters-{}.json",
            std::process::id()
        ));
        let mut old = ClientSettings::default();
        old.action_bar.edge_enabled = [true, false, false];
        old.action_bar.extra_rows[1].visible = true; // Bottom rows 1 and 3.
        old.action_bar.extra_rows[3].visible = true; // Only left row 2.
        old.action_bar.extra_rows[3].locked = true;
        old.action_bar.extra_rows[3].shortcuts[0] = Some(SlotShortcut {
            key: 4,
            modifiers: 2,
        });
        old.future_preferences.extend([
            (
                "action_bars.bars.bottom_visible".into(),
                FutureValue::Bool(false),
            ),
            (
                "action_bars.bars.left_visible".into(),
                FutureValue::Bool(true),
            ),
            (
                "action_bars.bars.right_visible".into(),
                FutureValue::Bool(true),
            ),
            ("action_bars.bars.right_rows".into(), FutureValue::Int(2)),
            ("sound.sound.master".into(), FutureValue::Int(40)),
        ]);
        let mut legacy = serde_json::to_value(&old)?;
        fs::write(&path, serde_json::to_vec(&legacy)?)?;
        let migrated = ClientSettings::load(&path)?;
        assert_eq!(migrated.action_bar.edge_enabled, [false, true, true]);
        assert_eq!(migrated.action_bar.visible_rows(), [0, 1, 2]);
        for row in 0..6 {
            assert_eq!(migrated.action_bar.row(row), old.action_bar.row(row));
        }
        assert_eq!(migrated.action_bar.shortcuts, old.action_bar.shortcuts);
        assert_eq!(migrated.future_preferences.len(), 1);
        assert_eq!(
            migrated.future_preferences["sound.sound.master"],
            FutureValue::Int(40)
        );
        migrated.save(&path)?;
        assert_eq!(ClientSettings::load(&path)?, migrated);
        let saved = fs::read(&path)?;
        assert!(saved.len() as u64 <= MAX_SETTINGS_BYTES);
        assert!(!String::from_utf8(saved)?.contains("bars.bottom_visible"));
        let mut reenabled = migrated.clone();
        reenabled.action_bar.edge_enabled[0] = true;
        reenabled.reconcile_active_hotkey_profile()?;
        assert!(reenabled.action_bar.row_is_visible(0));
        assert!(!reenabled.action_bar.row_is_visible(1));
        assert!(reenabled.action_bar.row_is_visible(2));
        reenabled.save(&path)?;
        assert_eq!(ClientSettings::load(&path)?, reenabled);
        legacy["future_preferences"]
            .as_object_mut()
            .ok_or("Expected future intent object")?
            .remove("action_bars.bars.left_visible");
        fs::write(&path, serde_json::to_vec(&legacy)?)?;
        let missing = ClientSettings::load(&path)?;
        assert!(!missing.action_bar.edge_enabled[1]); // No absent intent inferred.
        assert_eq!(missing.action_bar.row(4), old.action_bar.row(4));
        for invalid in [
            FutureValue::Int(0),
            FutureValue::Text("true".into()),
            FutureValue::Choice("on".into()),
        ] {
            legacy["future_preferences"]["action_bars.bars.bottom_visible"] =
                serde_json::to_value(invalid)?;
            let bytes = serde_json::to_vec(&legacy)?;
            fs::write(&path, &bytes)?;
            assert!(ClientSettings::load(&path).is_err());
            assert_eq!(fs::read(&path)?, bytes);
        }
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn expanded_chords_and_all_configurable_future_choices_fit_existing_file_bounds()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::{
            action_bar::{ACTION_BAR_ROWS, ACTION_BAR_SLOTS, SlotShortcut},
            settings_catalog::{
                FutureValue, Implementation, OptionKind, SETTINGS_SECTIONS, ShortcutBinding,
            },
        };
        let mut settings = ClientSettings {
            panel_shortcuts: crate::panel_catalog::ShortcutOrder::all().saved_ids(),
            ..Default::default()
        };
        for row_index in 0..ACTION_BAR_ROWS {
            let mut row = settings
                .action_bar
                .row(row_index)
                .ok_or("missing action row")?;
            row.visible = true;
            row.shortcuts = std::array::from_fn(|slot| {
                let index = row_index * ACTION_BAR_SLOTS + slot;
                Some(SlotShortcut {
                    key: 4 + (index % 36) as u16,
                    modifiers: (index / 36) as u8,
                })
            });
            settings.action_bar.set_row(row_index, row)?;
        }
        for section in SETTINGS_SECTIONS {
            for option in section.options {
                if !matches!(option.implementation, Implementation::Pending { .. }) {
                    continue;
                }
                let value = match option.kind {
                    OptionKind::Toggle => Some(FutureValue::Bool(true)),
                    OptionKind::Integer { min, max } => {
                        Some(FutureValue::Int(50_i32.clamp(min, max)))
                    }
                    OptionKind::Decimal { min, max } => {
                        Some(FutureValue::Decimal((min + max) / 2.0))
                    }
                    OptionKind::Text { max_bytes } => Some(FutureValue::Text(
                        "example".chars().take(max_bytes).collect(),
                    )),
                    OptionKind::Choice(choices) => choices
                        .first()
                        .map(|choice| FutureValue::Choice(choice.id.into())),
                    OptionKind::Binding => Some(FutureValue::Binding(ShortcutBinding {
                        key: 4,
                        ctrl: true,
                        alt: false,
                        shift: false,
                        meta: false,
                    })),
                    OptionKind::Action => None,
                };
                if let Some(value) = value {
                    settings
                        .future_preferences
                        .insert(format!("{}.{}", section.id, option.id), value);
                }
            }
        }
        settings.validate()?;
        let normal_bytes = serde_json::to_vec(&settings)?.len();
        assert!(
            normal_bytes as u64 <= MAX_SETTINGS_BYTES,
            "normal complete preferences are {normal_bytes} bytes"
        );
        // Also cover every text field at its established ASCII byte limit.
        for section in SETTINGS_SECTIONS {
            for option in section.options {
                if let OptionKind::Text { max_bytes } = option.kind {
                    let key = format!("{}.{}", section.id, option.id);
                    if let Some(FutureValue::Text(value)) =
                        settings.future_preferences.get_mut(&key)
                    {
                        *value = "x".repeat(max_bytes);
                    }
                }
            }
        }
        settings.validate()?;
        let max_text_bytes = serde_json::to_vec(&settings)?.len();
        assert!(
            max_text_bytes as u64 <= MAX_SETTINGS_BYTES,
            "complete preferences at text limits are {max_text_bytes} bytes"
        );
        for value in settings.future_preferences.values_mut() {
            if let FutureValue::Text(text) = value {
                // An escaped quote occupies one allowed raw byte and two JSON bytes.
                *text = "\"".repeat(text.len());
            }
        }
        settings.validate()?;
        let escaped_text_bytes = serde_json::to_vec(&settings)?.len();
        assert!(
            escaped_text_bytes as u64 <= MAX_SETTINGS_BYTES,
            "escaped text preferences are {escaped_text_bytes} bytes"
        );
        eprintln!(
            "complete preferences: {} future choices, {} chords; normal={normal_bytes} bytes, max_ascii_text={max_text_bytes} bytes, escaped_text={escaped_text_bytes} bytes",
            settings.future_preferences.len(),
            ACTION_BAR_ROWS * ACTION_BAR_SLOTS,
        );
        let path = std::env::temp_dir().join(format!(
            "oteryn-expanded-preferences-{}.json",
            std::process::id()
        ));
        settings.reconcile_active_hotkey_profile()?;
        settings.save(&path)?;
        assert_eq!(ClientSettings::load(&path)?, settings);
        fs::remove_file(path)?;
        Ok(())
    }
}
