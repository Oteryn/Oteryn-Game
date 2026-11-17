use crate::combat_input::PlayCommand;
use crate::play::{
    Outbox, PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::inventory::MoveResponse;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    pub fn container_move_all(&self, destination_view: u8) -> bool {
        move_all(&mut lock(&self.mailbox), &self.commands, destination_view)
    }
}
pub(in crate::play) fn move_all(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    destination_view: u8,
) -> bool {
    if mailbox.inventory_shutdown
        || mailbox.item_movement.pending
        || mailbox.item_movement.dragged.is_some()
        || mailbox.item_movement.prompt.is_some()
    {
        return false;
    }
    let Some(requests) = mailbox.containers.move_all(destination_view) else {
        return false;
    };
    if commands.send(PlayCommand::MoveContents(requests)).is_err() {
        mailbox.containers.bulk.pending = false;
        mailbox.containers.bulk.feedback = Some("Session ended");
        return false;
    }
    mailbox.item_movement.pending = true;
    true
}
impl Outbox {
    pub(in crate::play) fn container_bulk_result(&self, response: Option<MoveResponse>) -> bool {
        let mut mailbox = lock(&self.0);
        let more = mailbox.containers.bulk_result(response);
        mailbox.item_movement.pending = more;
        more
    }
}
