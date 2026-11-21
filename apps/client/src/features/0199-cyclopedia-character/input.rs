use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_character::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    /// True when a command was sent; false when the builder refused (for example, while pending).
    pub fn cyclopedia_character_request(&self, intent: Intent) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.cyclopedia_character;
        let command = if intent == Intent::Next
            && panel.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::Achievements
        {
            oteryn_client_ui::cyclopedia_character::achievement_next_request(panel)
        } else {
            request(panel, intent)
        };
        let Some(command) = command else {
            return false;
        };
        if self.commands.send(PlayCommand::Feature(command)).is_err() {
            mailbox.cyclopedia_character.pending = false;
            mailbox.cyclopedia_character.feedback = Some(tr!("The session has ended"));
            return false;
        }
        true
    }
    pub fn cyclopedia_character_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::cyclopedia_character::blessings_key(events) {
            #[cfg(windows)]
            if !self.uses_shared_frontend() {
                super::view::blessings::open_window(self);
            }
            if cfg!(not(windows)) || self.uses_shared_frontend() {
                lock(&self.mailbox).cyclopedia_character.blessings.open = true;
                self.blessings_request();
            }
            return true;
        }
        if !oteryn_client_ui::cyclopedia_character::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).cyclopedia_character.open = true;
        }
        true
    }
}
