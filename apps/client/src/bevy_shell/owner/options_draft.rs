use super::*;

impl HostOwner {
    pub fn open_graphics(&mut self) {
        if self.phase == Phase::Closed {
            return;
        }
        if self.modal_active() {
            return;
        }
        self.reset_input();
        self.settings.open = true;
        self.draft = Some(Draft {
            scope: self.scope,
            values: self.settings.preferences.clone(),
            baseline: self.settings.preferences.clone(),
        });
    }
    /// Global's ShowOptionsHotkeys: the options window, opened on its hotkey page.
    pub fn show_hotkey_options(&mut self) {
        self.open_graphics();
        if self.draft.is_some() {
            self.hotkey_options += 1;
        }
    }
    pub fn edit(&mut self, edit: impl FnOnce(&mut GraphicsDraft)) {
        let Some(mut graphics) = self.preferences_draft().map(GraphicsDraft::from) else {
            return;
        };
        edit(&mut graphics);
        self.edit_preferences(|values| {
            values.graphics.fullscreen = graphics.fullscreen;
            values.graphics.vsync = graphics.vsync;
            values.graphics.integer_scaling = graphics.integer_scaling;
        });
    }
    pub fn preferences_draft(&self) -> Option<&ReferencePreferences> {
        self.draft.as_ref().map(|draft| &draft.values)
    }
    pub fn edit_preferences(&mut self, edit: impl FnOnce(&mut ReferencePreferences)) {
        if let Some(draft) = &mut self.draft {
            edit(&mut draft.values);
        }
    }
    pub fn cancel(&mut self) {
        self.reset_input();
        self.draft = None;
        self.settings.open = false;
    }
    pub fn apply(&mut self, close: bool) {
        if self.settings.preference_transfer_pending() {
            self.settings.feedback =
                "Wait for the options transfer to finish before applying edits.".into();
            return;
        }
        let Some(mut draft) = self.draft.take() else {
            return;
        };
        if draft.scope != self.scope || self.phase == Phase::Closed {
            self.settings.open = false;
            return;
        }
        // The retained owner alone validates, saves and installs preferences. Invalid edits
        // remain in the transient draft; a disk failure still accepts the live values.
        let before = self.settings.preferences.miscellaneous;
        let after = draft.values.miscellaneous;
        let forget = before.stay_logged_in && !after.stay_logged_in
            || before.remember_device && !after.remember_device;
        if forget && draft.values.validate().is_ok() {
            if !self.forget_device() {
                self.draft = Some(draft);
                return;
            }
            draft.values.miscellaneous.remember_device = false;
        }
        self.settings.apply(draft.values.clone());
        if self.settings.preferences != draft.values {
            self.draft = Some(draft);
            return;
        }
        if close {
            self.settings.open = false;
        } else {
            self.open_graphics();
        }
    }
    pub(super) fn rebase_transferred_preferences(&mut self) {
        if let Some(draft) = &mut self.draft {
            let mut restored = self.settings.preferences.clone();
            if draft.values.miscellaneous != draft.baseline.miscellaneous {
                restored.miscellaneous = draft.values.miscellaneous;
            }
            match restored.rebase_edits(&draft.baseline, &draft.values) {
                Ok(()) => {
                    draft.baseline = self.settings.preferences.clone();
                    draft.values = restored;
                }
                Err(error) => {
                    self.settings.feedback =
                        format!("Could not carry draft edits across the transfer: {error}");
                    draft.baseline = self.settings.preferences.clone();
                    draft.values = self.settings.preferences.clone();
                }
            }
        }
    }
}

#[cfg(test)]
#[test]
fn restore_carries_only_edited_draft_fields_forward() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};

    bevy_tasks::IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
    });
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("preferences.json");
    let saved = ReferencePreferences {
        audio_device: "From backup".into(),
        ..Default::default()
    };
    crate::play::PreferenceStore::export_to(&saved, &path.with_extension("backup.json"))?;
    let mut owner = HostOwner::new(Some(path.clone()))?;
    owner.settings.use_background_preferences();
    owner.open_graphics();
    owner.settings.restore_backup()?;
    owner.edit_preferences(|draft| draft.graphics.fullscreen = true);
    owner.apply(false);
    assert!(
        owner
            .preferences_draft()
            .ok_or("draft")?
            .graphics
            .fullscreen
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while owner.settings.preference_save_pending() && Instant::now() < deadline {
        owner.poll([800, 600]);
        std::thread::yield_now();
    }
    assert!(!owner.settings.preference_save_pending());
    assert_eq!(
        owner.preferences_draft().ok_or("draft")?.audio_device,
        saved.audio_device
    );
    owner.apply(false);
    while owner.settings.preference_save_pending() && Instant::now() < deadline {
        owner.poll([800, 600]);
        std::thread::yield_now();
    }
    let actual = crate::play::PreferenceStore::read_transfer(&path)?;
    assert!(actual.graphics.fullscreen);
    assert_eq!(actual.audio_device, saved.audio_device);
    Ok(())
}
