use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::arena::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn arena_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.arena, intent)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.arena.pending = None;
            mailbox.arena.feedback = Some(tr!("The session has ended"));
        }
    }
    pub fn arena_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::arena::open_key(events) {
            return false;
        }
        self.arena_request(Intent::Catalogue);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).arena.open = true;
        }
        true
    }
}
