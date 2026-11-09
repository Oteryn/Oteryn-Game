//! Publish only confirmed bounded map facts from the selected read-only query.
use crate::play::Outbox;
use oteryn_client_runtime::personal_map_marks::PersonalMapMarks;
use oteryn_session::{CommandStatus, GatedCommandResult, SessionError};
impl Outbox {
    pub(in crate::play) fn personal_map_marks_initialize(&self) {
        crate::play::mailbox::lock(&self.0)
            .pushed
            .personal_map_marks = Some(PersonalMapMarks::default());
    }

    pub(in crate::play) fn personal_map_marks_result(
        &self,
        result: &GatedCommandResult,
    ) -> Result<(), SessionError> {
        let malformed = || SessionError::SessionUnusable;
        if !matches!(
            result.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            return if result.payload.is_empty() {
                Ok(())
            } else {
                Err(malformed())
            };
        }
        let marks = PersonalMapMarks::decode(&result.payload).ok_or_else(malformed)?;
        crate::play::mailbox::lock(&self.0)
            .pushed
            .personal_map_marks = Some(marks);
        Ok(())
    }
}
