use super::*;

impl SettingsState {
    pub fn sync_archive_bindings(&mut self, link: &crate::play::PlayLink) {
        let mut prefs = self.preferences.clone();
        if let Some(edit) =
            link.sync_archive_bindings(&mut prefs.archive_bindings, self.archive_edit)
        {
            self.apply(prefs);
            self.archive_edit = edit;
        }
    }

    pub fn sync_spellbook_filter(&mut self, link: &crate::play::PlayLink) {
        let mut prefs = self.preferences.clone();
        if let Some(edit) =
            link.sync_spellbook_filter(&mut prefs.spellbook_filter, self.spellbook_edit)
        {
            self.apply(prefs);
            self.spellbook_edit = edit;
        }
    }

    pub fn sync_skills_layout(&mut self, link: &crate::play::PlayLink) {
        let mut preferences = self.preferences.clone();
        if let Some(edit) =
            link.sync_skills_layout(&mut preferences.skills_layout, self.skills_edit)
        {
            self.apply(preferences);
            self.skills_edit = edit;
        }
    }

    pub fn sync_battle_lists(&mut self, link: &crate::play::PlayLink) {
        let mut preferences = self.preferences.clone();
        if let Some(edit) = link.sync_battle_list_state(
            &mut preferences.battle_lists,
            &mut preferences.active_battle_list,
            self.battle_edit,
        ) {
            self.apply(preferences);
            self.battle_edit = edit;
        }
    }
}

impl SettingsState {
    pub fn restore(
        &mut self,
        view: &mut PlayView,
        viewport: [u32; 2],
    ) -> Result<(), PreferenceError> {
        if view.local.minimap_edit > self.minimap_edit {
            let edit = view.local.minimap_edit;
            let mut preferences = self.preferences.clone();
            preferences.minimap = view.preferences().minimap.clone();
            self.apply(preferences);
            self.minimap_edit = edit;
        }
        view.local.minimap_edit = self.minimap_edit;
        view.set_world_viewport(viewport)?;
        if view.preferences() != &self.preferences {
            view.set_preferences(self.preferences.clone())?;
        }
        Ok(())
    }

    pub fn capture(&mut self, view: &mut PlayView) -> Result<(), PreferenceError> {
        if view.preferences().audio != self.preferences.audio {
            let mut preferences = self.preferences.clone();
            preferences.audio = view.preferences().audio;
            self.apply(preferences);
        }
        // Merge any binding/zoom edit made while the shortcut was being handled.
        if view.preferences() != &self.preferences {
            view.set_preferences(self.preferences.clone())?;
        }
        Ok(())
    }
}
