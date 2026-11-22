//! Session-choice sidecar writes follow successful main-file saves.
use super::{
    super::{MiscellaneousPreferences, ReferencePreferences},
    SettingsState,
};
use std::{path::Path, sync::Mutex};

type SessionBaseline = Mutex<MiscellaneousPreferences>;

impl SettingsState {
    #[cfg(bevy_shell)]
    pub(crate) fn disable_remember_device(&mut self) {
        use std::sync::atomic::Ordering;

        self.preferences.miscellaneous.remember_device = false;
        self.remember_epoch.fetch_add(1, Ordering::AcqRel);
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
                let current = crate::atomic_file::wait_for_lock(&path)
                    .and_then(|lock| disable_remember_device_locked(&path, &baseline, lock))
                    .map_err(|error| error.to_string());
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

#[cfg(bevy_shell)]
fn disable_remember_device_locked(
    path: &Path,
    baseline: &SessionBaseline,
    _lock: std::fs::File,
) -> std::io::Result<()> {
    let mut baseline = baseline
        .lock()
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let before = MiscellaneousPreferences {
        remember_device: true,
        ..*baseline
    };
    let after = MiscellaneousPreferences {
        remember_device: false,
        ..*baseline
    };
    after.publish_edits(before, path)?;
    *baseline = after;
    Ok(())
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
