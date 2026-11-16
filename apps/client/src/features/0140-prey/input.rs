use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::prey::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    pub fn prey_set_open(&self, open: bool) {
        lock(&self.mailbox).prey.open = open;
    }

    pub fn prey_select(&self, slot: u8, index: u8) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.prey.choose_candidate(slot, index) {
            enqueue(&mut mailbox, &self.commands, Intent::Select { slot });
        }
    }

    pub fn prey_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }

    pub fn prey_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::prey::open_key(events) {
            return false;
        }
        self.prey_request(Intent::Refresh);
        lock(&self.mailbox).prey.open = true;
        true
    }
}

pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(request) = request(&mut mailbox.prey, intent)
        && commands.send(PlayCommand::Prey(request)).is_err()
    {
        mailbox.prey.pending = None;
        mailbox.prey.feedback = Some(tr!("The session has ended."));
    }
}
