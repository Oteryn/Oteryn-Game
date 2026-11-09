use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_character::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn cyclopedia_character_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.cyclopedia_character, intent)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.cyclopedia_character.pending = false;
            mailbox.cyclopedia_character.feedback = Some("The session has ended");
        }
    }
    pub fn cyclopedia_character_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if oteryn_client_ui::cyclopedia_character::blessings_key(events) {
            #[cfg(windows)]
            super::view::blessings::open_window(self);
            #[cfg(not(windows))]
            {
                lock(&self.mailbox).cyclopedia_character.blessings.open = true;
                self.blessings_request();
            }
            return true;
        }
        if !oteryn_client_ui::cyclopedia_character::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).cyclopedia_character.open = true;
        }
        true
    }
}
