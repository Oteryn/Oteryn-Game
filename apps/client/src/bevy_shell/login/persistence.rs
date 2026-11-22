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
        self.start_save()
            .ok_or_else(|| self.form.message().to_owned())
    }

    pub(in crate::bevy_shell) fn start_save(&mut self) -> Option<crate::NativeLoginConfig> {
        if self.saving.is_some() || !self.form.can_submit() {
            return None;
        }
        let Some(path) = self.path.clone() else {
            self.form
                .feedback("Cannot locate the login configuration directory.".into());
            return None;
        };
        let visible = self.form.settings.clone();
        if let Err(error) = visible.validate() {
            self.form.feedback(error);
            return None;
        }
        match crate::atomic_file::try_lock(&path) {
            Ok(lock) => self.finish_save(visible.save_edits(&path, &self.baseline, lock)),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                let edited = visible.clone();
                let before = self.baseline.clone();
                let task = bevy_tasks::IoTaskPool::get_or_init(|| {
                    bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
                })
                .spawn(async move {
                    let lock =
                        crate::atomic_file::lock(&path).map_err(|error| error.to_string())?;
                    edited.save_edits(&path, &before, lock)
                });
                self.saving = Some(PendingSave {
                    task,
                    visible,
                    revision: self.revision,
                    assets: None,
                });
                self.form.feedback("Waiting to save login settings…".into());
                None
            }
            Err(_) => self.finish_save(Err(LoginSettings::save_error())),
        }
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
        self.saving = None;
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
