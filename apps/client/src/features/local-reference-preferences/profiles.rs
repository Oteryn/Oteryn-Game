//! Bounded named snapshots of the existing action bindings.
use super::{PreferenceError, ReferencePreferences};

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b" _-".contains(&byte))
}

impl ReferencePreferences {
    pub(super) fn validate_profiles(&self) -> Result<(), PreferenceError> {
        if self.hotkey_profiles.len() > 16
            || self.hotkey_profiles.keys().any(|name| !valid_name(name))
        {
            return Err(PreferenceError);
        }
        if self
            .custom_hotkey_profiles
            .keys()
            .chain(self.bar_hotkey_profiles.keys())
            .chain(self.bar_action_profiles.keys())
            .any(|name| !self.hotkey_profiles.contains_key(name))
        {
            return Err(PreferenceError);
        }
        let mut candidate = self.clone();
        for (name, bindings) in &self.hotkey_profiles {
            candidate.custom_hotkeys = self
                .custom_hotkey_profiles
                .get(name)
                .cloned()
                .unwrap_or_default();
            candidate.action_keys = self
                .bar_hotkey_profiles
                .get(name)
                .cloned()
                .unwrap_or_default();
            if self
                .bar_action_profiles
                .get(name)
                .is_some_and(|actions| !actions.valid())
            {
                return Err(PreferenceError);
            }
            candidate.keybindings.clone_from(bindings);
            candidate.validate_bindings()?;
        }
        Ok(())
    }

    pub fn save_hotkey_profile(&mut self, name: &str) -> Result<(), PreferenceError> {
        if !valid_name(name)
            || (self.hotkey_profiles.len() == 16 && !self.hotkey_profiles.contains_key(name))
        {
            return Err(PreferenceError);
        }
        self.validate_bindings()?;
        self.bar_hotkey_profiles
            .insert(name.into(), self.action_keys.clone());
        self.bar_action_profiles
            .insert(name.into(), self.bar_actions.clone());
        self.hotkey_profiles
            .insert(name.into(), self.keybindings.clone());
        self.custom_hotkey_profiles
            .insert(name.into(), self.custom_hotkeys.clone());
        Ok(())
    }

    pub fn load_hotkey_profile(&mut self, name: &str) -> Result<(), PreferenceError> {
        let bindings = self.hotkey_profiles.get(name).ok_or(PreferenceError)?;
        let mut candidate = self.clone();
        candidate.keybindings.clone_from(bindings);
        candidate.custom_hotkeys = self
            .custom_hotkey_profiles
            .get(name)
            .cloned()
            .unwrap_or_default();
        candidate.action_keys = self
            .bar_hotkey_profiles
            .get(name)
            .cloned()
            .unwrap_or_else(|| self.action_keys.clone());
        candidate.bar_actions = self
            .bar_action_profiles
            .get(name)
            .cloned()
            .unwrap_or_else(|| self.bar_actions.clone());
        candidate.validate_bindings()?;
        if !candidate.bar_actions.valid() {
            return Err(PreferenceError);
        }
        self.action_keys = candidate.action_keys;
        self.bar_actions = candidate.bar_actions;
        self.custom_hotkeys = candidate.custom_hotkeys;
        self.keybindings = candidate.keybindings;
        // Profiles are explicit binding snapshots; automatic presets must not overwrite them.
        self.chat_context_hotkeys = false;
        self.manual_hotkey_profile = true;
        Ok(())
    }
}

impl super::SettingsState {
    /// An explicit load wins over older unsynchronized edits, even when the saved values match.
    pub fn load_custom_hotkey_profile(&mut self, name: &str) -> Result<(), PreferenceError> {
        let mut preferences = self.preferences.clone();
        preferences.load_hotkey_profile(name)?;
        preferences.validate()?;
        self.apply(preferences);
        self.bar_actions_edit = crate::play::feature_0017_attack::view::next_edit();
        Ok(())
    }
}
