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
    #[cfg(bevy_shell)]
    pub(super) fn save_miscellaneous_in_background(
        &mut self,
        options: MiscellaneousPreferences,
    ) -> Result<(), String> {
        let path = self.base_path.as_ref()
            .ok_or_else(|| "Session options storage unavailable".to_owned())?
            .with_extension("session-options.json");
        let previous = self.session_save.take();
        self.session_save = Some(bevy_tasks::IoTaskPool::get().spawn(async move {
            let earlier = match previous {
                Some(previous) => previous.await,
                None => Ok(()),
            };
            earlier.and(options.save(&path).map_err(|error| error.to_string()))
        }));
        Ok(())
    }
}
