//! Applying local preferences to the live play view.
use super::{PreferenceError, ReferencePreferences};
use crate::play::PlayView;

impl PlayView {
    pub fn preference_character(&self) -> Option<[u8; 16]> {
        self.local.character
    }

    /// Installed only after native admission has accepted the selected character.
    pub(crate) fn set_preference_character(&mut self, character: [u8; 16]) {
        self.local.character = Some(character);
    }

    pub fn set_world_viewport(&mut self, viewport: [u32; 2]) -> Result<(), PreferenceError> {
        if viewport.contains(&0) || self.local.viewport == viewport {
            return Ok(());
        }
        let mut candidate = self.clone();
        candidate.local.viewport = viewport;
        candidate
            .apply_preference_scale()
            .map_err(|_| PreferenceError)?;
        *self = candidate;
        Ok(())
    }

    #[must_use]
    pub const fn preferences(&self) -> &ReferencePreferences {
        &self.local.preferences
    }

    pub fn set_preferences(
        &mut self,
        preferences: ReferencePreferences,
    ) -> Result<(), PreferenceError> {
        preferences.validate()?;
        let mut candidate = self.clone();
        candidate.local.preferences = preferences;
        candidate.rebuild().map_err(|_| PreferenceError)?;
        *self = candidate;
        Ok(())
    }
}
