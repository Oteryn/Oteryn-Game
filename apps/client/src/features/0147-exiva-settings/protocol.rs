use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn exiva_initialize(&self, selected: bool) {
        lock(&self.0).exiva.selected = selected;
    }
    pub(in crate::play) fn exiva_result(&self, result: &oteryn_session::GatedCommandResult) {
        let mut mailbox = lock(&self.0);
        mailbox.exiva.pending = false;
        if matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        ) {
            mailbox.exiva.apply(&result.payload);
        } else {
            mailbox.exiva.error = Some("Exiva request refused; refresh the list");
        }
    }
}
