use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::prey::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    pub fn prey_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn prey_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::prey::open_key(events) {
            return false;
        }
        self.prey_request(Intent::Refresh);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).prey.open = true;
        }
        true
    }
}

pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.prey, intent)
        && commands.send(PlayCommand::Prey(command)).is_err()
    {
        mailbox.prey.pending = false;
        mailbox.prey.feedback = Some("The session has ended");
    }
}
