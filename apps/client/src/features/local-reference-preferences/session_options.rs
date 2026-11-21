//! Session-choice sidecar writes follow successful main-file saves.
use super::{
    super::{MiscellaneousPreferences, ReferencePreferences},
    SettingsState,
};
use std::{path::Path, sync::Mutex};

type SessionBaseline = Mutex<MiscellaneousPreferences>;

impl SettingsState {
    pub(super) fn persist_preferences(
        &mut self,
        preferences: &ReferencePreferences,
    ) -> Result<(), String> {
        let store = self
            .store
            .as_mut()
            .ok_or_else(|| tr!("Preference storage is unavailable").to_owned())?;
        let path = self
            .base_path
            .as_ref()
            .ok_or_else(|| "Session options storage unavailable".to_owned())?
            .with_extension("session-options.json");
        let options = preferences.miscellaneous;
        #[cfg(bevy_shell)]
        if self.background_preferences || store.saving() || self.session_save.is_some() {
            if self.session_queued >= 8 {
                return Err("Session options writer is busy".into());
            }
            let (complete, saved) = tokio::sync::oneshot::channel();
            store
                .apply_then(preferences.clone(), move |success| {
                    let _ = complete.send(success);
                })
                .map_err(|error| error.to_string())?;
            self.session_queued += 1;
            let previous = self.session_save.take();
            let baseline = self.session_baseline.clone();
            self.session_save = Some(bevy_tasks::IoTaskPool::get().spawn(async move {
                let earlier = match previous {
                    Some(previous) => previous.await,
                    None => Ok(()),
                };
                let current = if saved.await == Ok(true) {
                    save_options(&baseline, options, &path)
                } else {
                    Ok(())
                };
                earlier.and(current)
            }));
            return Ok(());
        }
        store
            .apply(preferences.clone())
            .map_err(|error| error.to_string())?;
        save_options(&self.session_baseline, options, &path)
    }
}

fn save_options(
    baseline: &SessionBaseline,
    options: MiscellaneousPreferences,
    path: &Path,
) -> Result<(), String> {
    let mut previous = baseline.lock().map_err(|error| error.to_string())?;
    options
        .save_edits(*previous, path)
        .map_err(|error| error.to_string())?;
    *previous = options;
    Ok(())
}
