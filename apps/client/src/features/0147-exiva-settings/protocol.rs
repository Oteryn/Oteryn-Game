use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    /// Refresh after an accepted edit rather than inferring the server-owned list.
    pub(in crate::play) async fn exiva_run(
        &self,
        session: &mut impl crate::play::Stepper,
        payload: &[u8],
    ) -> Result<(), oteryn_session::SessionError> {
        let result = session.exiva(payload).await?;
        let result = if payload != [5] && result.payload == [1] {
            session.exiva(&[5]).await?
        } else {
            result
        };
        self.exiva_result(&result);
        Ok(())
    }

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
            mailbox.exiva.error = Some(tr!("Exiva request refused; refresh the list"));
        }
    }
}
