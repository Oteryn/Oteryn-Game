use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::animus_mastery::Intent;
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn animus_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) =
            oteryn_client_ui::animus_mastery::request(&mut mailbox.animus, intent)
            && self.commands.send(PlayCommand::Animus(payload)).is_err()
        {
            mailbox.animus.pending = None;
            mailbox.animus.error = Some("Session ended");
        }
    }

    pub fn animus_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::animus_mastery::open_key(events) {
            return false;
        }
        self.animus_request(Intent::Refresh);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).animus.open = true;
        }
        true
    }
}
