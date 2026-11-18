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
        let Some(draft) = self.draft.take() else {
            return;
        };
        if draft.scope != self.scope || self.phase == Phase::Closed {
            self.settings.open = false;
            return;
        }
        // The retained owner alone validates, saves and installs preferences. Invalid edits
        // remain in the transient draft; a disk failure still accepts the live values.
        let before = self.settings.preferences.miscellaneous;
        self.settings.apply(draft.values.clone());
        if self.settings.preferences != draft.values {
            self.draft = Some(draft);
            return;
        }
        let after = self.settings.preferences.miscellaneous;
        if before.stay_logged_in && !after.stay_logged_in
            || before.remember_device && !after.remember_device
        {
            self.forget_device();
        }
        if close {
            self.settings.open = false;
        } else {
            self.open_graphics();
        }
    }
}
