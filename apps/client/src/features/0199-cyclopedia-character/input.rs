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
            lock(&self.mailbox).cyclopedia_character.blessings.open = true;
            self.blessings_request();
            return true;
        }
        if !oteryn_client_ui::cyclopedia_character::open_key(events) {
            return false;
        }
        lock(&self.mailbox).cyclopedia_character.open = true;
        true
    }
    /// The accepted custody summary is remote, read-only, and combines Store and mail deliveries.
    pub fn store_inbox_set_open(&self, open: bool) {
        {
            let mut mailbox = lock(&self.mailbox);
            let panel = &mut mailbox.cyclopedia_character;
            panel.items.open = open;
            if !open {
                return;
            }
            // Never present a previous character/profile's holdings as this inbox.
            panel.items.page = None;
        }
        self.cyclopedia_character_request(Intent::Items);
    }
}

impl PlayLink {
    pub fn cyclopedia_holdings_filter(
        &self,
        search: &str,
        location: Option<oteryn_client_runtime::cyclopedia_character::items::Location>,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let Some(command) = oteryn_client_ui::cyclopedia_character::holdings_filter(
            &mut mailbox.cyclopedia_character,
            search,
            location,
        ) else {
            return false;
        };
        if self.commands.send(PlayCommand::Feature(command)).is_err() {
            mailbox.cyclopedia_character.pending = false;
            mailbox.cyclopedia_character.feedback = Some(tr!("The session has ended"));
            return false;
        }
        true
    }
}
