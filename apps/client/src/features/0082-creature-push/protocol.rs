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
                    "Creature pushed"
                }
                (_, [8, 2]) => "Creature cannot be moved",
                (_, [8, 3]) => "Push refused",
                (_, [8, 4]) => "Push not possible",
                (_, [8, 5]) => "Not enough room to push",
                (_, [8, 6]) => "Push destination out of reach",
                (_, [8, 7]) => "Push exhausted",
                (CommandStatus::Rejected, _) => "Push refused",
                _ => "Invalid push result",
            });
    }
}
