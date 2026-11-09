use super::*;

impl SettingsState {
    pub fn character_scope(&self) -> Option<[u8; 16]> {
        self.character
    }
    pub fn scope_label(&self) -> &'static str {
        if self.character.is_some() {
            "Character settings"
        } else {
            "Local defaults for new characters"
        }
    }

    /// The admitted actor identity selects a local character namespace; no account identity
    /// or credential is inferred from a window title or a character name.
    pub fn select_character(&mut self, character: Option<[u8; 16]>) {
        if character == self.character {
            return;
        }
        let Some(base) = self.base_path.as_ref() else {
            return;
        };
        let path = character.map_or_else(
            || base.clone(),
            |id| PreferenceStore::character_path(base, id),
        );
        let missing = !path.exists();
        let mut selected = Self::new(Some(path));
        if character.is_some() && missing {
            match PreferenceStore::load(base.clone()) {
                Ok(template) => selected.apply(template.preferences().clone()),
                Err(error) => {
                    selected.feedback =
                        format!("Character defaults loaded; could not read local template: {error}")
                }
            }
        }
        self.help_report_pending = false;
        self.store = selected.store;
        self.preferences = selected.preferences;
        self.feedback = selected.feedback;
        self.character = character;
        self.battle_edit = crate::play::feature_0017_attack::view::next_edit();
        self.skills_edit = self.battle_edit;
        self.item_hotkeys_edit = self.battle_edit;
        self.minimap_edit = self.battle_edit;
        self.spellbook_edit = self.battle_edit;
        self.archive_edit = self.battle_edit;
    }

}
