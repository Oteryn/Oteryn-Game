use super::*;
use std::ffi::OsString;

pub(super) struct PendingSave {
    task: bevy_tasks::Task<Result<LoginSettings, String>>,
    visible: LoginSettings,
    revision: u64,
    assets: Option<OsString>,
}

impl LoginState {
    #[cfg(test)]
    pub(in crate::bevy_shell) fn submit(&mut self) -> Result<crate::NativeLoginConfig, String> {
        self.show();
        self.start_save();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while self.save_pending() && std::time::Instant::now() < deadline {
            if let Some((config, _)) = self.poll_save() {
                return Ok(config);
            }
            std::thread::yield_now();
        }
        Err(self.form.message().to_owned())
    }

    pub(in crate::bevy_shell) fn start_save(&mut self) {
        if self.saving.is_some() || !self.form.can_submit() {
            return;
        }
        let Some(path) = self.path.clone() else {
            self.form
                .feedback("Cannot locate the login configuration directory.".into());
            return;
        };
        let visible = self.form.settings.clone();
        if let Err(error) = visible.validate() {
            self.form.feedback(error);
            return;
        }
        let edited = visible.clone();
        let before = self.baseline.clone();
        let task = bevy_tasks::IoTaskPool::get_or_init(|| {
            bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
        })
        .spawn(async move {
            let lock = crate::atomic_file::lock(&path).map_err(|_| LoginSettings::save_error())?;
            edited.save_edits(&path, &before, lock)
        });
        self.saving = Some(PendingSave {
            task,
            visible,
            revision: self.revision,
            assets: None,
        });
        self.form.feedback("Waiting to save login settings…".into());
    }

    pub(in crate::bevy_shell) fn poll_save(
        &mut self,
    ) -> Option<(crate::NativeLoginConfig, Option<OsString>)> {
        let saved = bevy_tasks::futures::check_ready(&mut self.saving.as_mut()?.task)?;
        let save = self.saving.take()?;
        if self.revision != save.revision || !self.visible || self.form.settings != save.visible {
            return None;
        }
        self.finish_save(saved).map(|config| (config, save.assets))
    }

    pub(in crate::bevy_shell) fn save_pending(&self) -> bool {
        self.saving.is_some()
    }

    pub(in crate::bevy_shell) fn set_pending_assets(&mut self, assets: Option<OsString>) {
        if let Some(save) = &mut self.saving {
            save.assets = assets;
        }
    }

    pub(in crate::bevy_shell) fn cancel_save(&mut self) {
        if self.saving.is_some() {
            self.revision = self.revision.saturating_add(1);
        }
    }

    fn finish_save(
        &mut self,
        saved: Result<LoginSettings, String>,
    ) -> Option<crate::NativeLoginConfig> {
        if let Err(error) = saved {
            self.form.feedback(error);
            return None;
        }
        self.baseline = self.form.settings.clone();
        self.form.submit().ok()
    }
}
