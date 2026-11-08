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
        let mut candidate = self.clone();
        for bindings in self.hotkey_profiles.values() {
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
        self.hotkey_profiles
            .insert(name.into(), self.keybindings.clone());
        Ok(())
    }

    pub fn load_hotkey_profile(&mut self, name: &str) -> Result<(), PreferenceError> {
        let bindings = self.hotkey_profiles.get(name).ok_or(PreferenceError)?;
        let mut candidate = self.clone();
        candidate.keybindings.clone_from(bindings);
        candidate.validate_bindings()?;
        self.keybindings = candidate.keybindings;
        Ok(())
    }
}
