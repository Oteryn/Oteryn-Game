use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_client_runtime::world_interaction::Command;
impl Outbox {
    pub(in crate::play) fn world_context_initialize(&self, look: bool, bed: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.world_context.look = look;
        mailbox.world_context.bed = bed;
    }
    pub(in crate::play) async fn world_context_run(
        &self,
        session: &mut impl Stepper,
        command: &Command,
    ) -> Result<(), oteryn_session::SessionError> {
        let result = session.world_context(command).await;
        let mut mailbox = lock(&self.0);
        mailbox.world_context.pending = false;
        match result {
            Ok(text) => {
                if command.action == oteryn_client_runtime::world_interaction::Action::Look {
                    mailbox.world_context.look_text = Some(text.clone());
                }
                mailbox.world_context.feedback = Some(text);
                Ok(())
            }
            Err(error) => {
                mailbox.world_context.feedback = Some("Session unavailable".into());
                Err(error)
            }
        }
    }
}
