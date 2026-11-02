use crate::play::{Outbox, mailbox::lock};

impl Outbox {
    pub(in crate::play) fn animus_initialize(&self, selected: bool) {
        lock(&self.0).animus.selected = selected;
    }

    pub(in crate::play) fn animus_result(&self, result: &oteryn_session::GatedCommandResult) {
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        let _ = lock(&self.0).animus.finish(accepted, &result.payload);
    }
}
