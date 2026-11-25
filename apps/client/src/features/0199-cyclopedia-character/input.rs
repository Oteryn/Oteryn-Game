use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_character::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    /// True when a command was sent; false when the builder refused (for example, while pending).
    pub fn cyclopedia_character_request(&self, intent: Intent) -> bool {
        let mut mailbox = lock(&self.mailbox);
        self.cyclopedia_character_request_locked(&mut mailbox, intent)
    }

    fn cyclopedia_character_request_locked(
        &self,
        mailbox: &mut crate::play::mailbox::Mailbox,
        intent: Intent,
    ) -> bool {
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
        if panel.items.pending_offset.is_some() {
            mailbox.store_inbox_refresh_pending = false;
        }
        if self.commands.send(PlayCommand::Feature(command)).is_err() {
            mailbox.cyclopedia_character.pending = false;
            mailbox.cyclopedia_character.own_vocation_pending = false;
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
    /// Open the remote custody summary and request the buyer's Store claim entries.
    pub fn store_inbox_set_open(&self, open: bool) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.cyclopedia_character;
        panel.items.open = open;
        if !open {
            // Keep rejecting the superseded holdings reply until it has arrived.
            mailbox.store_inbox_refresh_pending &= panel.items.pending_offset.is_some();
            mailbox.store_inbox_query_queued = false;
            return;
        }
        panel.items.page = None;
        panel.items.location = None;
        panel.items.search.clear();
        if panel.pending || panel.blessings.pending {
            mailbox.store_inbox_refresh_pending = true;
        } else {
            mailbox.store_inbox_refresh_pending = false;
            self.cyclopedia_character_request_locked(&mut mailbox, Intent::Items);
        }
        mailbox.store.inbox = Default::default();
        mailbox.store.inbox_page_cursor = None;
        self.store_inbox_refresh_locked(&mut mailbox);
    }

    pub(in crate::play) fn pump_store_inbox_refresh(&self) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.store_inbox_query_queued && !mailbox.store.pending {
            mailbox.store_inbox_query_queued = false;
            if mailbox.cyclopedia_character.items.open {
                self.store_inbox_refresh_locked(&mut mailbox);
            }
        }
        let panel = &mailbox.cyclopedia_character;
        if mailbox.store_inbox_refresh_pending && !panel.pending && !panel.blessings.pending {
            let open = panel.items.open;
            mailbox.store_inbox_refresh_pending = false;
            if open {
                self.cyclopedia_character_request_locked(&mut mailbox, Intent::Items);
            }
        }
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
        mailbox.store_inbox_refresh_pending = false;
        if self.commands.send(PlayCommand::Feature(command)).is_err() {
            mailbox.cyclopedia_character.pending = false;
            mailbox.cyclopedia_character.feedback = Some(tr!("The session has ended"));
            return false;
        }
        true
    }
}
