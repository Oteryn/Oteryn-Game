//! Session-choice sidecar writes follow successful main-file saves.
use super::{
    super::{MiscellaneousPreferences, ReferencePreferences},
    SettingsState,
};
use std::{path::Path, sync::Mutex};

type SessionBaseline = Mutex<MiscellaneousPreferences>;

impl SettingsState {
    #[cfg(bevy_shell)]
    pub(crate) fn record_device_opt_out(&self) -> impl FnOnce() -> Result<(), String> + use<> {
        let path = self.base_path.as_ref().map(|p| p.with_extension("session-options.json"));
        let epoch = self.remember_epoch.clone();
        move || {
            epoch.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            if let Some(path) = path {
                MiscellaneousPreferences::record_opt_out(&path).map_err(|error| error.to_string())?;
            }
            Ok(())
        }
    }

    #[cfg(bevy_shell)]
    pub(crate) fn finish_device_opt_out(&mut self) {
        self.preferences.miscellaneous.remember_device = false;
        let Some(base) = &self.base_path else {
            return;
        };
        let path = base.with_extension("session-options.json");
        let baseline = self.session_baseline.clone();
        let previous = self.session_save.take();
        self.session_save = Some(
            bevy_tasks::IoTaskPool::get_or_init(|| {
                bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
            })
            .spawn(async move {
                let earlier = match previous {
                    Some(previous) => previous.await,
                    None => Ok(()),
                };
                let current = MiscellaneousPreferences::try_replay_opt_out(&path)
                    .map_err(|error| error.to_string())
                    .and_then(|()| {
                        baseline
                            .lock()
                            .map_err(|error| error.to_string())?
                            .remember_device = false;
                        Ok(())
                    });
                earlier.and(current)
            }),
        );
    }

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
                .apply_then(preferences.clone(), true, move |success| {
                    let _ = complete.send(success);
                })
                .map_err(|error| error.to_string())?;
            self.session_queued += 1;
            let previous = self.session_save.take();
            let baseline = self.session_baseline.clone();
            let epoch = self.remember_epoch.clone();
            let submitted = epoch.load(std::sync::atomic::Ordering::Acquire);
            self.session_save = Some(bevy_tasks::IoTaskPool::get().spawn(async move {
                let earlier = match previous {
                    Some(previous) => previous.await,
                    None => Ok(()),
                };
                let current = if saved.await == Ok(true) {
                    save_options(&baseline, options, &path, || {
                        submitted == epoch.load(std::sync::atomic::Ordering::Acquire)
                    })
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
        save_options(&self.session_baseline, options, &path, || true)
    }
}

fn save_options(
    baseline: &SessionBaseline,
    mut options: MiscellaneousPreferences,
    path: &Path,
    keep_remember_device: impl FnOnce() -> bool,
) -> Result<(), String> {
    let _lock = crate::atomic_file::lock(path).map_err(|error| error.to_string())?;
    let mut previous = baseline.lock().map_err(|error| error.to_string())?;
    let mut before = *previous;
    if !keep_remember_device() {
        // A save queued before explicit sign-out must not opt the device back in.
        before.remember_device = true;
        options.remember_device = false;
    }
    options
        .publish_edits(before, path)
        .map_err(|error| error.to_string())?;
    *previous = options;
    Ok(())
}

#[cfg(test)]
#[test]
fn opt_out_after_epoch_check_wins_the_merged_write() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("options.json");
    let before = MiscellaneousPreferences::default();
    let edited = MiscellaneousPreferences {
        remember_device: true,
        confirm_exit: false,
        ..before
    };
    let mut recorded = None;
    save_options(&Mutex::new(before), edited, &path, || {
        recorded = Some(MiscellaneousPreferences::record_opt_out(&path));
        true
    })?;
    recorded.ok_or("epoch check did not run")??;
    let persisted: MiscellaneousPreferences = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert!(!persisted.remember_device);
    assert!(!persisted.confirm_exit);
    assert!(!path.with_added_extension("opt-out").exists());
    Ok(())
}

#[cfg(test)]
#[test]
fn failed_merged_publication_keeps_opt_out_for_replay() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("options.json");
    let before = MiscellaneousPreferences {
        remember_device: true,
        ..Default::default()
    };
    before.save(&path)?;
    MiscellaneousPreferences::record_opt_out(&path)?;
    crate::atomic_file::fail_durable_rename_to(&path);
    assert!(save_options(&Mutex::new(before), before, &path, || true).is_err());
    assert!(path.with_added_extension("opt-out").exists());
    assert!(!MiscellaneousPreferences::read(&path)?.remember_device);
    Ok(())
}
