use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::depot::{Intent, open_key, request_for_selection, request_in_mode};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    pub fn depot_open_container(&self, handle: oteryn_session::ItemHandle) {
        enqueue_container(&mut lock(&self.mailbox), &self.commands, handle);
    }
    pub fn depot_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn depot_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !open_key(events) {
            return false;
        }
        self.open_depot();
        true
    }
}

pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let manual_sort = mailbox.container_manual_sort.active();
    if let Some((intent, slot)) = request_in_mode(&mut mailbox.depot, intent, manual_sort)
        && commands
            .send(PlayCommand::Depot(request_for_selection(
                &mailbox.depot,
                intent,
                slot,
            )))
            .is_err()
    {
        mailbox.depot.pending = false;
        mailbox.depot.feedback = tr!("The session has ended");
    }
}

pub(in crate::play) fn enqueue_container(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    handle: oteryn_session::ItemHandle,
) {
    if let Some(handle) = oteryn_client_ui::depot::request_container(&mut mailbox.depot, handle)
        && commands.send(PlayCommand::DepotContainer(handle)).is_err()
    {
        mailbox.depot.pending = false;
        mailbox.depot.feedback = tr!("The session has ended");
    }
}
