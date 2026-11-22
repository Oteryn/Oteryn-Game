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
        if self.save_pending() || !self.form.can_submit() {
            return;
        }
        let Some(path) = self.path.clone() else {
            self.form
                .feedback("Cannot locate the login configuration directory.".into());
            return;
        };
        self.record_edits();
        let fields_to_save = std::array::from_fn(|i| self.edited_fields[i] || self.env_fields[i]);
        let visible = self.form.settings.clone();
        if let Err(error) = visible.validate() {
            self.form.feedback(error);
            return;
        }
        let edited = visible.clone();
        let previous = self.saving.take();
        let task = bevy_tasks::IoTaskPool::get_or_init(|| {
            bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
        })
        .spawn(async move {
            // Each submission writes its form values after every earlier writer has finished.
            if let Some(previous) = previous {
                let _ = previous.task.await;
            }
            edited.save_fields(&path, fields_to_save)
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
        self.saving
            .as_ref()
            .is_some_and(|save| save.revision == self.revision)
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
        let settings = match saved {
            Ok(settings) => settings,
            Err(error) => {
                self.form.feedback(error);
                return None;
            }
        };
        if self.form.settings != settings {
            self.revision = self.revision.saturating_add(1);
        }
        self.shown_settings = settings.clone();
        self.form.settings = settings;
        self.form.submit().ok()
    }
}
