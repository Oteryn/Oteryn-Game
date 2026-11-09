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
        if let Some(edit) = link.sync_battle_lists(&mut preferences.battle_lists, self.battle_edit)
        {
            self.apply(preferences);
            self.battle_edit = edit;
        }
    }
}
