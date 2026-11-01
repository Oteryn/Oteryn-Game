use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::party::{Intent, request};

impl PlayLink {
    pub fn party_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.party, intent)
            && self.commands.send(PlayCommand::Party(command)).is_err()
        {
            mailbox.party.pending = None;
            mailbox.party.feedback = Some("The session has ended");
        }
    }

    pub fn open_party(&self) {
        self.party_request(Intent::Refresh);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).party.open = true;
        }
    }
}
