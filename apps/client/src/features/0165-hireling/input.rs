use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::hireling::{Intent, request};

impl PlayLink {
    pub fn hireling_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.hireling, intent)
            && self.commands.send(PlayCommand::Hireling(command)).is_err()
        {
            mailbox.hireling.pending = None;
            mailbox.hireling.feedback = Some("The session has ended");
        }
    }

    pub fn hireling_target(&self, x: u16, y: u16, floor: i8) {
        lock(&self.mailbox).hireling.target = Some((x, y, floor));
    }
}
