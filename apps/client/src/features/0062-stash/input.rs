use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::stash::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn stash_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.stash, intent)
            && self.commands.send(PlayCommand::Stash(command)).is_err()
        {
            mailbox.stash.pending = false;
            mailbox.stash.feedback = Some("The session has ended".into());
        }
    }
    pub fn stash_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::stash::open_key(events) {
            return false;
        }
        self.stash_request(Intent::Refresh);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).stash.open = true;
        }
        true
    }
}
