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
const SETTINGS_VERSION: u8 = 3;
const DEFAULT_FRAME_LIMIT: u16 = 120;
const LEGACY_UNLIMITED_FRAME_LIMIT: u16 = 60;
const MIN_FRAME_LIMIT: u16 = 1;
const MAX_FRAME_LIMIT: u16 = 200;

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
    /// Last non-zero foreground limit, retained while `fps` is zero.
    #[serde(default)]
    pub last_frame_limit: u16,
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
            version: SETTINGS_VERSION,
            english: false,
            fullscreen: false,
            window_width: 900,
            window_height: 620,
            vsync: true,
            fps: DEFAULT_FRAME_LIMIT,
            last_frame_limit: DEFAULT_FRAME_LIMIT,
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
    #[must_use]
    pub fn stored_bool(&self, key: &str) -> Option<bool> {
        match self.future_preferences.get(key) {
            Some(crate::settings_catalog::FutureValue::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn stored_int(&self, key: &str) -> Option<i32> {
        match self.future_preferences.get(key) {
            Some(crate::settings_catalog::FutureValue::Int(value)) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn stored_choice(&self, key: &str) -> Option<&str> {
        match self.future_preferences.get(key) {
            Some(crate::settings_catalog::FutureValue::Choice(value)) => Some(value),
            _ => None,
        }
    }

    /// Toggle the reference FPS overlay. An unset value has the same visible
    /// behavior as `false`, so the first invocation always enables it.
    pub fn toggle_fps_indicator(&mut self) -> bool {
        let enabled = !self
            .stored_bool("graphics.graphics.fps_indicator")
            .unwrap_or(false);
        self.future_preferences.insert(
            "graphics.graphics.fps_indicator".into(),
            crate::settings_catalog::FutureValue::Bool(enabled),
        );
        enabled
    }

    #[must_use]
    pub const fn frame_limit_enabled(&self) -> bool {
        self.fps != 0
    }

    /// Toggle the foreground scheduler limit without discarding its value.
    pub fn set_frame_limit_enabled(&mut self, enabled: bool) {
        if enabled {
            if self.fps == 0 {
                self.fps = self.last_frame_limit;
            }
        } else if self.fps != 0 {
            self.last_frame_limit = self.fps;
            self.fps = 0;
        }
    }

    /// Change the retained foreground limit. The live scheduler follows it
    /// immediately unless the no-limit switch is active.
    pub fn set_frame_limit(&mut self, limit: u16) -> io::Result<()> {
        if !(MIN_FRAME_LIMIT..=MAX_FRAME_LIMIT).contains(&limit) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid foreground frame limit",
            ));
        }
        self.last_frame_limit = limit;
        if self.fps != 0 {
            self.fps = limit;
        }
        Ok(())
    }

    /// Foreground and background rates consumed by the native redraw scheduler.
    #[must_use]
    pub const fn redraw_fps(&self, focused: bool) -> u16 {
        if focused {
            self.fps
        } else {
            self.background_fps
        }
    }

    /// An unset preference preserves the pre-settings behavior: atlas frame on,
    /// translucent highlight off.
    #[must_use]
    pub fn target_frame_enabled(&self) -> bool {
        !matches!(
            self.stored_choice("game_window.window.target_marking"),
            Some("highlight_only" | "none")
        )
    }

    #[must_use]
    pub fn target_highlight_enabled(&self) -> bool {
        matches!(
            self.stored_choice("game_window.window.target_marking"),
            Some("frame_and_highlight" | "highlight_only")
        )
    }

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

    /// `Food and Beverages` and `Move Item` were initially stored under the UI
    /// Sounds page. They belong to Sound in the reference client. Consume the
    /// old spelling once, while preserving an explicitly stored new value.
    fn migrate_legacy_ui_sound_preferences(&mut self) {
        for (old, new) in [
            ("ui_sounds.sound.eating", "sound.sound.eating"),
            ("ui_sounds.sound.item_movement", "sound.sound.item_movement"),
        ] {
            if let Some(value) = self.future_preferences.remove(old) {
                self.future_preferences.entry(new.into()).or_insert(value);
            }
        }
    }

    /// Version 2 files predate a durable remembered limit. A limited legacy
    /// file already contains the remembered value; an unlimited one used the
    /// old UI's 60 FPS fallback when the switch was turned back on.
    fn migrate_legacy_frame_limit(&mut self, legacy_wide_range: bool) {
        if self.fps != 0 {
            if legacy_wide_range {
                self.fps = self.fps.clamp(MIN_FRAME_LIMIT, MAX_FRAME_LIMIT);
            }
            self.last_frame_limit = self.fps;
        } else if self.last_frame_limit == 0 {
            self.last_frame_limit = LEGACY_UNLIMITED_FRAME_LIMIT;
        } else if legacy_wide_range {
            self.last_frame_limit = self
                .last_frame_limit
                .clamp(MIN_FRAME_LIMIT, MAX_FRAME_LIMIT);
        }
    }

    fn migrate_settings_version(&mut self) -> io::Result<()> {
        match self.version {
            1 => {
                self.reconcile_active_hotkey_profile()?;
                self.hotkeys
                    .migrate_v1_reference_defaults(&self.movement_keys);
                self.action_bar = self
                    .hotkeys
                    .active()
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "Missing hotkey profile")
                    })?
                    .action_bar
                    .clone();
                self.version = SETTINGS_VERSION;
                Ok(())
            }
            2 => {
                self.hotkeys.migrate_v2_reference_defaults();
                self.version = SETTINGS_VERSION;
                Ok(())
            }
            SETTINGS_VERSION => Ok(()),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported client preferences version",
            )),
        }
    }

    pub(crate) fn reconcile_active_hotkey_profile(&mut self) -> io::Result<()> {
        let active = self
            .hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing hotkey profile"))?;
        active.action_bar = self.action_bar.clone();
        Ok(())
    }

    /// Validate the hotkey editor against the currently edited action-bar
    /// bindings before Apply commits the draft.
    pub fn validate_hotkey_draft(&self) -> io::Result<()> {
        let mut hotkeys = self.hotkeys.clone();
        hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing hotkey profile"))?
            .action_bar = self.action_bar.clone();
        hotkeys.validate(&self.movement_keys)
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

    /// Select the next existing profile without inventing one. The current
    /// profile retains its live action-bar configuration before the switch.
    pub fn select_next_hotkey_profile(&mut self) -> io::Result<bool> {
        if self.hotkeys.profiles.len() <= 1 {
            return Ok(false);
        }
        let next = (self.hotkeys.selected + 1) % self.hotkeys.profiles.len();
        self.select_hotkey_profile(next)?;
        Ok(true)
    }

    /// Select the previous existing profile without inventing one. The current
    /// profile retains its live action-bar configuration before the switch.
    pub fn select_previous_hotkey_profile(&mut self) -> io::Result<bool> {
        let len = self.hotkeys.profiles.len();
        if len <= 1 {
            return Ok(false);
        }
        let previous = (self.hotkeys.selected + len - 1) % len;
        self.select_hotkey_profile(previous)?;
        Ok(true)
    }

    /// Select the profile named exactly like the admitted character when the
    /// observed Auto-Switch option is enabled. No profile is created or guessed.
    /// The previous profile's current action bars are retained before switching.
    pub fn auto_switch_hotkey_profile(&mut self, character_name: &str) -> io::Result<bool> {
        if !self.hotkeys.auto_switch {
            return Ok(false);
        }
        let candidate = character_name.trim();
        let Some(index) = self
            .hotkeys
            .profiles
            .iter()
            .position(|profile| profile.name.eq_ignore_ascii_case(candidate))
        else {
            return Ok(false);
        };
        if index == self.hotkeys.selected {
            return Ok(false);
        }
        self.select_hotkey_profile(index)?;
        Ok(true)
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
        if self.version != SETTINGS_VERSION
            || !(800..=3840).contains(&self.window_width)
            || !(600..=2160).contains(&self.window_height)
            || !(self.fps == 0 || (MIN_FRAME_LIMIT..=MAX_FRAME_LIMIT).contains(&self.fps))
            || !(MIN_FRAME_LIMIT..=MAX_FRAME_LIMIT).contains(&self.last_frame_limit)
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
        self.validate_hotkey_draft()?;
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
        let legacy_wide_frame_range = settings.version < SETTINGS_VERSION;
        settings.migrate_settings_version()?;
        settings.migrate_legacy_frame_limit(legacy_wide_frame_range);
        settings.migrate_legacy_action_rows()?;
        settings.migrate_legacy_action_edge_masters()?;
        settings.migrate_legacy_ui_sound_preferences();
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
            let action = [
                "movement.north",
                "movement.east",
                "movement.south",
                "movement.west",
            ][index];
            if self.hotkeys.profile_owns_movement_action(action) {
                return None;
            }
            [StepDir::North, StepDir::East, StepDir::South, StepDir::West]
                .get(index)
                .copied()
        })
    }

    /// Effective first cardinal bindings shown by the supplementary movement
    /// editor. `None` is an intentional profile-owned clear, not a fallback.
    #[must_use]
    pub fn effective_movement_keys(&self) -> [Option<u16>; 4] {
        const ACTIONS: [&str; 4] = [
            "movement.north",
            "movement.east",
            "movement.south",
            "movement.west",
        ];
        let active = self.hotkeys.active();
        std::array::from_fn(|index| {
            active
                .and_then(|profile| profile.general_chat_off.get(ACTIONS[index]))
                .map_or(Some(self.movement_keys[index]), |pair| {
                    pair.first.map(|binding| binding.key)
                })
        })
    }

    /// Rebind one cardinal direction in the active Chat Off profile and keep
    /// the legacy fallback synchronized. The change is atomic and validated.
    pub fn set_movement_key(&mut self, index: usize, key: u16) -> io::Result<()> {
        let action = [
            "movement.north",
            "movement.east",
            "movement.south",
            "movement.west",
        ]
        .get(index)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Unknown direction"))?;
        let mut candidate = self.clone();
        candidate.movement_keys[index] = key;
        candidate
            .hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing hotkey profile"))?
            .general_chat_off
            .insert(
                (*action).into(),
                crate::hotkeys::HotkeyPair {
                    first: Some(crate::settings_catalog::ShortcutBinding {
                        key,
                        ctrl: false,
                        alt: false,
                        shift: false,
                        meta: false,
                    }),
                    second: None,
                },
            );
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    /// Apply a complete cardinal preset through the same validated profile path.
    pub fn set_movement_keys(&mut self, keys: [u16; 4]) -> io::Result<()> {
        let mut candidate = self.clone();
        candidate.movement_keys = keys;
        let profile = candidate
            .hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing hotkey profile"))?;
        for (action, key) in [
            "movement.north",
            "movement.east",
            "movement.south",
            "movement.west",
        ]
        .into_iter()
        .zip(keys)
        {
            profile.general_chat_off.insert(
                action.into(),
                crate::hotkeys::HotkeyPair {
                    first: Some(crate::settings_catalog::ShortcutBinding {
                        key,
                        ctrl: false,
                        alt: false,
                        shift: false,
                        meta: false,
                    }),
                    second: None,
                },
            );
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
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
        settings.version = SETTINGS_VERSION.saturating_add(1);
        assert!(settings.validate().is_err());
    }

    #[test]
    fn v1_settings_load_save_reload_reference_defaults_without_inventing_stop()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy = [30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 68, 69];
        let mut settings = ClientSettings {
            version: 1,
            ..ClientSettings::default()
        };
        for (shortcut, key) in settings.action_bar.shortcuts.iter_mut().zip(legacy) {
            *shortcut = Some(crate::action_bar::SlotShortcut { key, modifiers: 0 });
        }
        settings.action_bar.chat_on_shortcuts.fill(None);
        let profile = settings
            .hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::other("missing default profile"))?;
        profile.general_chat_on.clear();
        profile.general_chat_off.clear();
        profile.general_chat_off.insert(
            "movement.north".into(),
            crate::hotkeys::HotkeyPair {
                first: Some(crate::settings_catalog::ShortcutBinding {
                    key: 26,
                    ctrl: true,
                    alt: false,
                    shift: false,
                    meta: false,
                }),
                second: None,
            },
        );
        profile.action_bar = settings.action_bar.clone();
        let mut inactive = crate::hotkeys::HotkeyProfile {
            name: "Inactive".into(),
            ..crate::hotkeys::HotkeyProfile::default()
        };
        inactive.general_chat_on.clear();
        inactive.general_chat_off.clear();
        inactive.action_bar = settings.action_bar.clone();
        settings.hotkeys.profiles.push(inactive);

        let path = std::env::temp_dir().join(format!(
            "oteryn-settings-v1-hotkeys-{}.json",
            std::process::id()
        ));
        fs::write(&path, serde_json::to_vec(&settings)?)?;
        let settings = ClientSettings::load(&path)?;
        assert_eq!(settings.version, SETTINGS_VERSION);
        assert_eq!(
            settings.action_bar.shortcuts[0].map(|key| key.key),
            Some(58)
        );
        assert_eq!(
            settings.action_bar.shortcuts[9].map(|key| key.key),
            Some(67)
        );
        let profile = settings
            .hotkeys
            .active()
            .ok_or_else(|| io::Error::other("missing default profile"))?;
        assert!(!profile.general_chat_on.contains_key("client.stop"));
        assert!(!profile.general_chat_off.contains_key("client.stop"));
        assert_eq!(
            profile.general_chat_off["movement.north"].first,
            Some(crate::settings_catalog::ShortcutBinding {
                key: 26,
                ctrl: false,
                alt: false,
                shift: false,
                meta: false,
            })
        );
        assert_eq!(
            profile.action_bar.shortcuts[11].map(|key| key.key),
            Some(69)
        );
        assert_eq!(
            profile.action_bar.chat_on_shortcuts[11].map(|key| key.key),
            Some(69)
        );
        assert_eq!(
            settings.hotkeys.profiles[1].action_bar.shortcuts[0].map(|key| key.key),
            Some(58)
        );
        assert!(
            !settings.hotkeys.profiles[1]
                .general_chat_off
                .contains_key("client.stop")
        );
        settings.validate()?;
        settings.save(&path)?;
        assert_eq!(ClientSettings::load(&path)?, settings);
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn v2_single_stop_defaults_migrate_to_the_exact_reference_matrices()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy_stop = crate::hotkeys::HotkeyPair {
            first: Some(crate::settings_catalog::ShortcutBinding {
                key: 41,
                ctrl: false,
                alt: false,
                shift: false,
                meta: false,
            }),
            second: None,
        };
        let mut settings = ClientSettings {
            version: 2,
            ..ClientSettings::default()
        };
        let profile = settings
            .hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::other("missing default profile"))?;
        profile.general_chat_on =
            std::collections::BTreeMap::from([("client.stop".into(), legacy_stop)]);
        profile.general_chat_off =
            std::collections::BTreeMap::from([("client.stop".into(), legacy_stop)]);
        let path = std::env::temp_dir().join(format!(
            "oteryn-settings-v2-reference-hotkeys-{}.json",
            std::process::id()
        ));

        let mut raw_v2 = serde_json::to_value(settings)?;
        raw_v2
            .as_object_mut()
            .ok_or_else(|| io::Error::other("settings must be an object"))?
            .remove("last_frame_limit");
        fs::write(&path, serde_json::to_vec(&raw_v2)?)?;
        let restored = ClientSettings::load(&path)?;
        let profile = restored
            .hotkeys
            .active()
            .ok_or_else(|| io::Error::other("missing restored profile"))?;
        let expected = crate::hotkeys::HotkeyPreferences::default();
        let expected = expected
            .active()
            .ok_or_else(|| io::Error::other("missing expected profile"))?;
        assert_eq!(profile.general_chat_on.len(), 38);
        assert_eq!(profile.general_chat_off.len(), 49);
        assert_eq!(profile.general_chat_on, expected.general_chat_on);
        assert_eq!(profile.general_chat_off, expected.general_chat_off);
        assert_eq!(
            profile.general_chat_on["chat.send"],
            crate::hotkeys::HotkeyPair {
                first: Some(crate::settings_catalog::ShortcutBinding {
                    key: 40,
                    ctrl: false,
                    alt: false,
                    shift: false,
                    meta: false,
                }),
                second: Some(crate::settings_catalog::ShortcutBinding {
                    key: 88,
                    ctrl: false,
                    alt: false,
                    shift: false,
                    meta: false,
                }),
            }
        );
        assert_eq!(
            profile.general_chat_off["movement.north"]
                .first
                .map(|binding| binding.key),
            Some(26)
        );
        restored.save(&path)?;
        assert_eq!(ClientSettings::load(&path)?, restored);
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn v2_customized_and_explicitly_cleared_maps_are_not_backfilled()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = ClientSettings {
            version: 2,
            ..ClientSettings::default()
        };
        let profile = settings
            .hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::other("missing default profile"))?;
        profile.general_chat_on.clear();
        profile.general_chat_off = std::collections::BTreeMap::from([(
            "client.stop".into(),
            crate::hotkeys::HotkeyPair::default(),
        )]);
        let mut inactive = crate::hotkeys::HotkeyProfile {
            name: "Customized".into(),
            ..crate::hotkeys::HotkeyProfile::default()
        };
        inactive.general_chat_on = std::collections::BTreeMap::from([(
            "client.options".into(),
            crate::hotkeys::HotkeyPair {
                first: Some(crate::settings_catalog::ShortcutBinding {
                    key: 30,
                    ctrl: false,
                    alt: false,
                    shift: false,
                    meta: false,
                }),
                second: None,
            },
        )]);
        inactive.general_chat_off.clear();
        settings.hotkeys.profiles.push(inactive);
        let expected = settings
            .hotkeys
            .profiles
            .iter()
            .map(|profile| {
                (
                    profile.general_chat_on.clone(),
                    profile.general_chat_off.clone(),
                )
            })
            .collect::<Vec<_>>();
        let path = std::env::temp_dir().join(format!(
            "oteryn-settings-v2-custom-hotkeys-{}.json",
            std::process::id()
        ));

        let mut raw_v2 = serde_json::to_value(settings)?;
        raw_v2
            .as_object_mut()
            .ok_or_else(|| io::Error::other("settings must be an object"))?
            .remove("last_frame_limit");
        fs::write(&path, serde_json::to_vec(&raw_v2)?)?;
        let restored = ClientSettings::load(&path)?;
        for (profile, (expected_on, expected_off)) in restored.hotkeys.profiles.iter().zip(expected)
        {
            assert_eq!(profile.general_chat_on, expected_on);
            assert_eq!(profile.general_chat_off, expected_off);
        }
        restored.save(&path)?;
        assert_eq!(ClientSettings::load(&path)?, restored);
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn action_bar_arrows_conflict_only_with_effective_movement_fallbacks()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = ClientSettings::default();
        settings.action_bar.shortcuts[12] = Some(crate::action_bar::SlotShortcut {
            key: settings.movement_keys[0],
            modifiers: 0,
        });
        assert!(
            settings
                .hotkeys
                .active_movement_fallback_keys(&settings.movement_keys)
                .is_empty()
        );
        settings.validate()?;

        settings
            .hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::other("missing default profile"))?
            .general_chat_off
            .remove("movement.north");
        assert_eq!(
            settings
                .hotkeys
                .active_movement_fallback_keys(&settings.movement_keys),
            vec![settings.movement_keys[0]]
        );
        assert!(settings.validate().is_err());
        Ok(())
    }

    #[test]
    fn v1_action_default_collision_preserves_legacy_chords_and_other_settings()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy = [30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 68, 69];
        let mut settings = ClientSettings {
            version: 1,
            english: true,
            ..ClientSettings::default()
        };
        for (shortcut, key) in settings.action_bar.shortcuts.iter_mut().zip(legacy) {
            *shortcut = Some(crate::action_bar::SlotShortcut { key, modifiers: 0 });
        }
        settings.action_bar.chat_on_shortcuts.fill(None);
        let profile = settings
            .hotkeys
            .active_mut()
            .ok_or_else(|| io::Error::other("missing default profile"))?;
        profile.action_bar = settings.action_bar.clone();
        profile.general_chat_off.insert(
            "client.options".into(),
            crate::hotkeys::HotkeyPair {
                first: Some(crate::settings_catalog::ShortcutBinding {
                    key: 58,
                    ctrl: false,
                    alt: false,
                    shift: false,
                    meta: false,
                }),
                second: None,
            },
        );
        let path = std::env::temp_dir().join(format!(
            "oteryn-settings-v1-hotkey-collision-{}.json",
            std::process::id()
        ));
        fs::write(&path, serde_json::to_vec(&settings)?)?;
        let migrated = ClientSettings::load(&path)?;
        assert!(migrated.english);
        assert_eq!(
            migrated.action_bar.shortcuts[0].map(|key| key.key),
            Some(30)
        );
        assert_eq!(
            migrated.hotkeys.profiles[0].general_chat_off["client.options"]
                .first
                .map(|binding| binding.key),
            Some(58)
        );
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn v1_modified_movement_cannot_take_another_directions_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_input_actions::Modifiers;

        let mut settings = ClientSettings {
            version: 1,
            ..ClientSettings::default()
        };
        let east_fallback = settings.movement_keys[1];
        let chat_off = &mut settings
            .hotkeys
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off;
        chat_off.clear();
        chat_off.insert(
            "movement.north".into(),
            crate::hotkeys::HotkeyPair {
                first: Some(crate::settings_catalog::ShortcutBinding {
                    key: east_fallback,
                    ctrl: true,
                    alt: false,
                    shift: false,
                    meta: false,
                }),
                second: None,
            },
        );
        let path = std::env::temp_dir().join(format!(
            "oteryn-settings-v1-movement-fallback-{}.json",
            std::process::id()
        ));
        fs::write(&path, serde_json::to_vec(&settings)?)?;

        let migrated = ClientSettings::load(&path)?;
        assert_eq!(
            migrated
                .hotkeys
                .active()
                .ok_or("default profile")?
                .general_chat_off["movement.north"],
            crate::hotkeys::HotkeyPair::default()
        );
        let east = NormalizedInputEvent::Key {
            code: KeyCode::new(migrated.movement_keys[1])?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        assert_eq!(migrated.direction(&[east]), Some(StepDir::East));
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn target_marking_maps_all_observed_choices_without_inventing_an_unset_default() {
        let mut settings = ClientSettings::default();
        assert!(settings.target_frame_enabled());
        assert!(!settings.target_highlight_enabled());
        for (choice, frame, highlight) in [
            ("frame_and_highlight", true, true),
            ("frame_only", true, false),
            ("highlight_only", false, true),
            ("none", false, false),
        ] {
            settings.future_preferences.insert(
                "game_window.window.target_marking".into(),
                crate::settings_catalog::FutureValue::Choice(choice.into()),
            );
            assert_eq!(settings.target_frame_enabled(), frame, "{choice}");
            assert_eq!(settings.target_highlight_enabled(), highlight, "{choice}");
        }
    }

    #[test]
    fn fps_indicator_hotkey_toggle_has_a_real_persisted_setting() {
        let mut settings = ClientSettings::default();
        assert_eq!(
            settings.stored_bool("graphics.graphics.fps_indicator"),
            None
        );
        assert!(settings.toggle_fps_indicator());
        assert_eq!(
            settings.stored_bool("graphics.graphics.fps_indicator"),
            Some(true)
        );
        assert!(!settings.toggle_fps_indicator());
        assert_eq!(
            settings.stored_bool("graphics.graphics.fps_indicator"),
            Some(false)
        );
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn legacy_frame_limit_files_gain_a_durable_restore_value() -> io::Result<()> {
        let path = std::env::temp_dir().join(format!(
            "oteryn-legacy-frame-limit-{}.json",
            std::process::id()
        ));
        for (fps, expected_last, enabled) in
            [(144, 144, true), (0, LEGACY_UNLIMITED_FRAME_LIMIT, false)]
        {
            let mut legacy = serde_json::to_value(ClientSettings {
                fps,
                ..ClientSettings::default()
            })?;
            legacy
                .as_object_mut()
                .ok_or_else(|| io::Error::other("settings must be an object"))?
                .remove("last_frame_limit");
            fs::write(&path, serde_json::to_vec(&legacy)?)?;
            let loaded = ClientSettings::load(&path)?;
            assert_eq!(loaded.last_frame_limit, expected_last);
            assert_eq!(loaded.frame_limit_enabled(), enabled);
            loaded.validate()?;
        }
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn v2_frame_limits_migrate_to_the_observed_one_through_two_hundred_range() -> io::Result<()> {
        let path =
            std::env::temp_dir().join(format!("oteryn-v2-frame-limit-{}.json", std::process::id()));
        for (fps, expected_fps, expected_last) in [
            (360, 200, 200),
            (0, 0, LEGACY_UNLIMITED_FRAME_LIMIT),
            (1, 1, 1),
        ] {
            let mut legacy = serde_json::to_value(ClientSettings {
                version: 2,
                fps,
                ..ClientSettings::default()
            })?;
            legacy
                .as_object_mut()
                .ok_or_else(|| io::Error::other("settings must be an object"))?
                .remove("last_frame_limit");
            fs::write(&path, serde_json::to_vec(&legacy)?)?;
            let loaded = ClientSettings::load(&path)?;
            assert_eq!(loaded.version, SETTINGS_VERSION);
            assert_eq!(loaded.fps, expected_fps);
            assert_eq!(loaded.last_frame_limit, expected_last);
        }
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn frame_limit_setter_enforces_the_observed_slider_bounds() {
        let mut settings = ClientSettings::default();
        assert!(settings.set_frame_limit(1).is_ok());
        assert_eq!(settings.fps, 1);
        assert!(settings.set_frame_limit(200).is_ok());
        assert_eq!(settings.fps, 200);
        assert!(settings.set_frame_limit(0).is_err());
        assert!(settings.set_frame_limit(201).is_err());
        assert_eq!(settings.fps, 200);
    }

    #[test]
    fn legacy_ui_sound_keys_move_once_and_explicit_sound_values_win()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::settings_catalog::FutureValue;

        let path = std::env::temp_dir().join(format!(
            "oteryn-legacy-ui-sounds-{}.json",
            std::process::id()
        ));
        let mut legacy = ClientSettings::default();
        legacy.future_preferences.extend([
            ("ui_sounds.sound.eating".into(), FutureValue::Bool(false)),
            (
                "ui_sounds.sound.item_movement".into(),
                FutureValue::Bool(false),
            ),
            ("sound.sound.item_movement".into(), FutureValue::Bool(true)),
        ]);
        fs::write(&path, serde_json::to_vec(&legacy)?)?;

        let migrated = ClientSettings::load(&path)?;
        assert_eq!(
            migrated.future_preferences.get("sound.sound.eating"),
            Some(&FutureValue::Bool(false))
        );
        assert_eq!(
            migrated.future_preferences.get("sound.sound.item_movement"),
            Some(&FutureValue::Bool(true))
        );
        assert!(
            !migrated
                .future_preferences
                .contains_key("ui_sounds.sound.eating")
        );
        assert!(
            !migrated
                .future_preferences
                .contains_key("ui_sounds.sound.item_movement")
        );

        migrated.save(&path)?;
        assert_eq!(ClientSettings::load(&path)?, migrated);
        let saved = String::from_utf8(fs::read(&path)?)?;
        assert!(!saved.contains("ui_sounds.sound.eating"));
        assert!(!saved.contains("ui_sounds.sound.item_movement"));
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn next_profile_cycles_existing_profiles_and_preserves_action_bars()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = ClientSettings::default();
        assert!(!settings.select_next_hotkey_profile()?);
        let mut second = crate::hotkeys::HotkeyProfile {
            name: "Second".into(),
            ..crate::hotkeys::HotkeyProfile::default()
        };
        second.action_bar.edge_enabled[0] = false;
        settings.hotkeys.profiles.push(second);
        settings.action_bar.edge_enabled[1] = false;

        assert!(settings.select_next_hotkey_profile()?);
        assert_eq!(settings.hotkeys.selected, 1);
        assert!(!settings.action_bar.edge_enabled[0]);
        assert!(!settings.hotkeys.profiles[0].action_bar.edge_enabled[1]);
        assert!(settings.select_next_hotkey_profile()?);
        assert_eq!(settings.hotkeys.selected, 0);
        assert!(!settings.action_bar.edge_enabled[1]);
        Ok(())
    }

    #[test]
    fn previous_profile_wraps_and_preserves_outgoing_action_bars()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = ClientSettings::default();
        assert!(!settings.select_previous_hotkey_profile()?);
        let mut second = crate::hotkeys::HotkeyProfile {
            name: "Second".into(),
            ..crate::hotkeys::HotkeyProfile::default()
        };
        second.action_bar.edge_enabled[0] = false;
        settings.hotkeys.profiles.push(second);
        settings.action_bar.edge_enabled[1] = false;

        assert!(settings.select_previous_hotkey_profile()?);
        assert_eq!(settings.hotkeys.selected, 1);
        assert!(!settings.action_bar.edge_enabled[0]);
        assert!(!settings.hotkeys.profiles[0].action_bar.edge_enabled[1]);
        assert!(settings.select_previous_hotkey_profile()?);
        assert_eq!(settings.hotkeys.selected, 0);
        assert!(!settings.action_bar.edge_enabled[1]);
        Ok(())
    }

    #[test]
    fn hotkey_auto_switch_uses_only_an_existing_character_named_profile()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = ClientSettings::default();
        let mut knight = crate::hotkeys::HotkeyProfile {
            name: "Owned Knight".into(),
            ..Default::default()
        };
        knight.action_bar.edge_enabled = [false, true, false];
        settings.hotkeys.profiles.push(knight.clone());
        settings.hotkeys.auto_switch = true;
        assert!(!settings.auto_switch_hotkey_profile("Unknown")?);
        assert_eq!(settings.hotkeys.selected, 0);
        assert!(settings.auto_switch_hotkey_profile("owned knight")?);
        assert_eq!(settings.hotkeys.selected, 1);
        assert_eq!(settings.action_bar, knight.action_bar);
        assert!(!settings.auto_switch_hotkey_profile("Owned Knight")?);
        settings.hotkeys.auto_switch = false;
        assert!(!settings.auto_switch_hotkey_profile("Default")?);
        assert_eq!(settings.hotkeys.selected, 1);
        settings.validate()?;
        Ok(())
    }

    #[test]
    fn modified_action_shortcut_does_not_also_walk() -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_input_actions::{Modifier, Modifiers};
        let mut settings = ClientSettings {
            movement_keys: [26, 7, 22, 4],
            ..ClientSettings::default()
        };
        settings
            .hotkeys
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off
            .clear();
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
    fn profile_movement_presence_disables_the_legacy_global_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = ClientSettings {
            movement_keys: [26, 7, 22, 4],
            ..ClientSettings::default()
        };
        settings
            .hotkeys
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off
            .clear();
        let event = NormalizedInputEvent::Key {
            code: KeyCode::new(26)?,
            state: ButtonState::Pressed,
            modifiers: oteryn_input_actions::Modifiers::NONE,
            repeat: false,
        };
        assert_eq!(
            settings.direction(std::slice::from_ref(&event)),
            Some(StepDir::North)
        );
        settings
            .hotkeys
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off
            .insert(
                "movement.north".into(),
                crate::hotkeys::HotkeyPair::default(),
            );
        settings.validate()?;
        assert_eq!(settings.direction(&[event]), None);
        let east = NormalizedInputEvent::Key {
            code: KeyCode::new(7)?,
            state: ButtonState::Pressed,
            modifiers: oteryn_input_actions::Modifiers::NONE,
            repeat: false,
        };
        assert_eq!(settings.direction(&[east]), Some(StepDir::East));
        Ok(())
    }

    #[test]
    fn movement_editor_rebinds_the_active_profile_and_runtime_resolver()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::hotkeys::ResolvedHotkey;
        use oteryn_input_actions::Modifiers;

        let mut settings = ClientSettings::default();
        assert_eq!(
            settings.effective_movement_keys(),
            [Some(26), Some(7), Some(22), Some(4)]
        );
        settings.set_movement_keys([82, 79, 81, 80])?;
        assert_eq!(settings.movement_keys, [82, 79, 81, 80]);
        assert_eq!(
            settings.effective_movement_keys(),
            [Some(82), Some(79), Some(81), Some(80)]
        );
        let north = NormalizedInputEvent::Key {
            code: KeyCode::new(82)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        assert_eq!(
            settings.hotkeys.resolve(&[north], false),
            vec![ResolvedHotkey::General("movement.north".into())]
        );
        let old_north = NormalizedInputEvent::Key {
            code: KeyCode::new(26)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        assert!(settings.hotkeys.resolve(&[old_north], false).is_empty());
        settings.set_movement_key(0, 27)?;
        assert_eq!(settings.effective_movement_keys()[0], Some(27));
        assert!(settings.set_movement_key(0, 79).is_err());
        assert_eq!(settings.effective_movement_keys()[0], Some(27));
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
        settings.fps = MAX_FRAME_LIMIT.saturating_add(1);
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
                    key: 100 + (index % 36) as u16,
                    modifiers: (index / 36) as u8,
                })
            });
            settings.action_bar.set_row(row_index, row)?;
        }
        for section in SETTINGS_SECTIONS {
            for option in section.options {
                if !matches!(
                    option.implementation,
                    Implementation::Stored { .. } | Implementation::Pending { .. }
                ) {
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
