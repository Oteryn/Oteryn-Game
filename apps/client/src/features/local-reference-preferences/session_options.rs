//! Session-choice sidecar writes belong to the same acceptance owner as other preferences.
use super::{super::MiscellaneousPreferences, SettingsState};
impl SettingsState {
    pub(super) fn save_miscellaneous(
        &self,
        options: MiscellaneousPreferences,
    ) -> Result<(), String> {
        let base = self
            .base_path
            .as_ref()
            .ok_or_else(|| "Session options storage unavailable".to_owned())?;
        options
            .save(&base.with_extension("session-options.json"))
            .map_err(|error| error.to_string())
    }
}
