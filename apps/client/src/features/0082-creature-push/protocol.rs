use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
impl Outbox {
    pub(in crate::play) fn creature_push_initialize(&self, selected: bool) {
        lock(&self.0).creature_push_selected = selected;
    }
    pub(in crate::play) fn creature_push_result(&self, response: &ServerMechanicResponse) {
        lock(&self.0).movement_feedback =
            Some(match (response.status, response.payload.as_slice()) {
                (CommandStatus::Accepted | CommandStatus::DuplicateReplay, [8, 1]) => {
                    tr!("Creature pushed")
                }
                (_, [8, 2]) => tr!("Creature cannot be moved"),
                (_, [8, 3]) => tr!("Push refused"),
                (_, [8, 4]) => tr!("Push not possible"),
                (_, [8, 5]) => tr!("Not enough room to push"),
                (_, [8, 6]) => tr!("Push destination out of reach"),
                (_, [8, 7]) => tr!("Push exhausted"),
                (CommandStatus::Rejected, _) => tr!("Push refused"),
                _ => tr!("Invalid push result"),
            });
    }
}
