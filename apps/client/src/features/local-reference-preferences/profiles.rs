//! The public keybinding maps are the active editor context. Other contexts are captured
//! before a switch, preserving compatibility with the original single-map settings file.
use super::{PreferenceError, ReferencePreferences};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
type KeyMap = BTreeMap<String, String>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BindingSet {
    pub primary: BTreeMap<String, String>,
    pub secondary: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HotkeyPreset {
    pub chat_on: BindingSet,
    pub chat_off: BindingSet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HotkeyProfiles {
    pub selected: String,
    /// Editor context; actual input routing uses the live chat-window state.
    pub chat_on: bool,
    pub presets: BTreeMap<String, HotkeyPreset>,
}

impl Default for HotkeyProfiles {
    fn default() -> Self {
        Self {
            selected: "Default".into(),
            chat_on: false,
            presets: BTreeMap::from([("Default".into(), HotkeyPreset::default())]),
        }
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name.trim() == name
        && !name.chars().any(char::is_control)
}

impl HotkeyProfiles {
    pub(super) fn valid_shape(&self) -> bool {
        (1..=16).contains(&self.presets.len())
            && self.presets.contains_key(&self.selected)
            && self.presets.keys().enumerate().all(|(i, name)| {
                valid_name(name)
                    && !self
                        .presets
                        .keys()
                        .take(i)
                        .any(|old| old.eq_ignore_ascii_case(name))
            })
    }
}

impl ReferencePreferences {
    fn capture_hotkeys(&mut self) -> Result<(), PreferenceError> {
        let preset = self
            .hotkey_profiles
            .presets
            .get_mut(&self.hotkey_profiles.selected)
            .ok_or(PreferenceError)?;
        let context = if self.hotkey_profiles.chat_on {
            &mut preset.chat_on
        } else {
            &mut preset.chat_off
        };
        context.primary.clone_from(&self.keybindings);
        context.secondary.clone_from(&self.secondary_keybindings);
        Ok(())
    }

    fn load_hotkeys(&mut self) -> Result<(), PreferenceError> {
        let preset = self
            .hotkey_profiles
            .presets
            .get(&self.hotkey_profiles.selected)
            .ok_or(PreferenceError)?;
        let context = if self.hotkey_profiles.chat_on {
            &preset.chat_on
        } else {
            &preset.chat_off
        };
        self.keybindings.clone_from(&context.primary);
        self.secondary_keybindings.clone_from(&context.secondary);
        Ok(())
    }

    pub(super) fn binding_maps(
        &self,
        chat_on: bool,
    ) -> Result<(&KeyMap, &KeyMap), PreferenceError> {
        if chat_on == self.hotkey_profiles.chat_on {
            return Ok((&self.keybindings, &self.secondary_keybindings));
        }
        let preset = self
            .hotkey_profiles
            .presets
            .get(&self.hotkey_profiles.selected)
            .ok_or(PreferenceError)?;
        let context = if chat_on {
            &preset.chat_on
        } else {
            &preset.chat_off
        };
        Ok((&context.primary, &context.secondary))
    }

    pub fn select_hotkey_context(&mut self, chat_on: bool) -> Result<(), PreferenceError> {
        let mut candidate = self.clone();
        candidate.capture_hotkeys()?;
        candidate.hotkey_profiles.chat_on = chat_on;
        candidate.load_hotkeys()?;
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }

    pub fn select_hotkey_profile(&mut self, name: &str) -> Result<(), PreferenceError> {
        let mut candidate = self.clone();
        candidate.capture_hotkeys()?;
        candidate.hotkey_profiles.selected = name.to_owned();
        candidate.load_hotkeys()?;
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }

    pub fn add_hotkey_profile(&mut self, name: &str) -> Result<(), PreferenceError> {
        if !valid_name(name)
            || self.hotkey_profiles.presets.len() >= 16
            || self
                .hotkey_profiles
                .presets
                .keys()
                .any(|old| old.eq_ignore_ascii_case(name))
        {
            return Err(PreferenceError);
        }
        let mut candidate = self.clone();
        candidate.capture_hotkeys()?;
        let current = candidate
            .hotkey_profiles
            .presets
            .get(&candidate.hotkey_profiles.selected)
            .ok_or(PreferenceError)?
            .clone();
        candidate
            .hotkey_profiles
            .presets
            .insert(name.to_owned(), current);
        candidate.hotkey_profiles.selected = name.to_owned();
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }

    pub fn rename_hotkey_profile(&mut self, old: &str, name: &str) -> Result<(), PreferenceError> {
        if !valid_name(name)
            || self
                .hotkey_profiles
                .presets
                .keys()
                .any(|key| key != old && key.eq_ignore_ascii_case(name))
        {
            return Err(PreferenceError);
        }
        let mut candidate = self.clone();
        candidate.capture_hotkeys()?;
        let preset = candidate
            .hotkey_profiles
            .presets
            .remove(old)
            .ok_or(PreferenceError)?;
        candidate
            .hotkey_profiles
            .presets
            .insert(name.to_owned(), preset);
        if candidate.hotkey_profiles.selected == old {
            candidate.hotkey_profiles.selected = name.to_owned();
        }
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }

    pub fn remove_hotkey_profile(&mut self, name: &str) -> Result<(), PreferenceError> {
        if self.hotkey_profiles.presets.len() <= 1 {
            return Err(PreferenceError);
        }
        let mut candidate = self.clone();
        candidate.capture_hotkeys()?;
        candidate
            .hotkey_profiles
            .presets
            .remove(name)
            .ok_or(PreferenceError)?;
        if candidate.hotkey_profiles.selected == name {
            candidate.hotkey_profiles.selected = candidate
                .hotkey_profiles
                .presets
                .keys()
                .next()
                .ok_or(PreferenceError)?
                .clone();
            candidate.load_hotkeys()?;
        }
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }
}

impl ReferencePreferences {
    pub(super) fn validate_profiles(&self) -> Result<(), PreferenceError> {
        if self.hotkey_profiles.valid_shape() {
            Ok(())
        } else {
            Err(PreferenceError)
        }
    }
    pub fn save_hotkey_profile(&mut self, name: &str) -> Result<(), PreferenceError> {
        if !valid_name(name)
            || (self.hotkey_profiles.presets.len() >= 16
                && !self.hotkey_profiles.presets.contains_key(name))
        {
            return Err(PreferenceError);
        }
        let mut candidate = self.clone();
        candidate.capture_hotkeys()?;
        let current = candidate
            .hotkey_profiles
            .presets
            .get(&candidate.hotkey_profiles.selected)
            .ok_or(PreferenceError)?
            .clone();
        candidate
            .hotkey_profiles
            .presets
            .insert(name.into(), current);
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }
    pub fn load_hotkey_profile(&mut self, name: &str) -> Result<(), PreferenceError> {
        self.select_hotkey_profile(name)
    }
}
