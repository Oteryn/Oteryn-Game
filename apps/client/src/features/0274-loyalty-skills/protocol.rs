use crate::play::{Outbox, mailbox::lock};

impl Outbox {
    pub(in crate::play) fn loyalty_skills_initialize(&self, selected: bool) {
        lock(&self.0).loyalty_skills.selected = selected;
    }

    pub(in crate::play) fn loyalty_skills_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let accepted = matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        );
        lock(&self.0)
            .loyalty_skills
            .finish(accepted, &result.payload);
    }
}
