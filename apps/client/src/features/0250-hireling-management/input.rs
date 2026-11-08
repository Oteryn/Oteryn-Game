#[cfg(any(windows, test))]
#[path = "input/configuration.rs"]
pub(in crate::play) mod configuration;
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::hireling_management::{Intent, request};
impl PlayLink {
    pub fn hireling_management_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.hireling_management, intent)
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.hireling_management.pending = None;
            mailbox.hireling_management.feedback = Some("The session has ended");
        }
    }
}
