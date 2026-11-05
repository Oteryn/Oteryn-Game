use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::party::PartyResponse;

impl Outbox {
    pub(in crate::play) fn party_initialize(&self, selected: bool, chat_selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.party.selected = selected;
        mailbox.party.chat_selected = chat_selected;
    }

    pub(in crate::play) fn party_result(&self, response: &PartyResponse) {
        oteryn_client_ui::party::apply_result(&mut lock(&self.0).party, response);
    }
}

impl Outbox {
    pub(in crate::play) async fn party_run(
        &self,
        session: &mut impl crate::play::Stepper,
        request: &oteryn_client_runtime::party::PartyCommand,
    ) -> Result<(), oteryn_session::SessionError> {
        let response = if let Some(enabled) = request.shared_setting() {
            let outcome = session
                .chat_public(&oteryn_client_runtime::party::shared_intent(enabled))
                .await?;
            let view = session
                .party(&oteryn_client_runtime::party::PartyCommand::query())
                .await?;
            oteryn_client_runtime::party::shared_response(&outcome, view.view)
        } else {
            session.party(request).await?
        };
        self.party_result(&response);
        Ok(())
    }
}
