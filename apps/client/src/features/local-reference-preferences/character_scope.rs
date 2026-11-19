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
        let mut selected = Self::new(Some(path.clone()));
        #[cfg(bevy_shell)]
        let missing = match self.outgoing_stores.iter().position(|store| store.at_path(&path)) {
            Some(index) => {
                let store = self.outgoing_stores.swap_remove(index);
                selected.preferences = store.preferences().clone();
                selected.store = Some(store);
                selected.feedback.clear();
                false
            }
            None => missing,
        };
        #[cfg(bevy_shell)]
        if let Some(store) = self.store.take().filter(PreferenceStore::saving) {
            self.outgoing_stores.push(store);
        }
        // Session choices remain under the original local base across actor namespaces.
        selected.base_path = Some(base.clone());
        selected.preferences.miscellaneous = self.preferences.miscellaneous;
        if character.is_some() && missing {
            match PreferenceStore::load(base.clone()) {
                Ok(template) => {
                    let mut inherited = template.preferences().clone();
                    inherited.miscellaneous = self.preferences.miscellaneous;
                    selected.apply(inherited);
                }
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
        // The next play frame restores this character's saved chat mode.
        self.chat_mode_seen = None;
        self.battle_edit = crate::play::feature_0017_attack::view::next_edit();
        self.skills_edit = self.battle_edit;
        self.item_hotkeys_edit = self.battle_edit;
        self.bar_actions_edit = self.battle_edit;
        self.minimap_edit = self.battle_edit;
        self.hud_edit = self.battle_edit;
        self.spellbook_edit = self.battle_edit;
        self.stash_sort_edit = self.battle_edit;
        self.archive_edit = self.battle_edit;
    }
}

#[cfg(all(test, bevy_shell))]
#[test]
fn leaving_character_reports_its_failed_mark_save() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};

    bevy_tasks::IoTaskPool::get_or_init(|| bevy_tasks::TaskPoolBuilder::new().num_threads(1).build());
    let folder = tempfile::tempdir()?;
    let base = folder.path().join("preferences.json");
    let identity = [1; 16];
    let mut settings = SettingsState::new(Some(base.clone()));
    settings.select_character(Some(identity));
    std::fs::write(PreferenceStore::character_path(&base, identity), "damaged")?;
    settings.apply_minimap_marks_in_background(settings.preferences.clone());
    settings.select_character(None);
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        let _ = settings.poll_background_save();
        if settings.feedback.contains("could not save") {
            return Ok(());
        }
        std::thread::yield_now();
    }
    Err("the outgoing preference save error was lost".into())
}
