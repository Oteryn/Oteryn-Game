use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::party::PartyResponse;

impl Outbox {
    pub(in crate::play) fn party_initialize(&self, selected: bool, chat_selected: bool) {
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.party;
        panel.selected = selected;
        panel.chat_selected = chat_selected;
        panel.open = false;
        panel.pending = None;
        panel.state = None;
        panel.received_at = None;
        panel.selected_member = None;
        panel.selected_invitation = None;
        panel.list_order.clear();
        panel.awaiting_shared_feedback = false;
        panel.server_feedback = None;
        panel.feedback = None;
    }

    pub(in crate::play) fn party_result(&self, response: &PartyResponse) {
        let mut mailbox = lock(&self.0);
        let members = |panel: &oteryn_client_ui::party::PartyPanel| {
            panel.state.as_ref().map(|state| {
                state
                    .members
                    .iter()
                    .map(|member| member.handle)
                    .collect::<std::collections::BTreeSet<_>>()
            })
        };
        let before = members(&mailbox.party);
        oteryn_client_ui::party_list::apply_result(&mut mailbox.party, response);
        let after = members(&mailbox.party);
        let shown: std::collections::BTreeSet<_> = mailbox
            .analyser
            .party
            .iter()
            .map(|member| member.handle)
            .collect();
        if (mailbox.analyser.party_pending && before != after)
            || (!shown.is_empty() && after.as_ref().is_some_and(|members| members != &shown))
        {
            mailbox.analyser.party.clear();
            mailbox.analyser.party_revision = mailbox.analyser.party_revision.saturating_add(1);
            mailbox.analyser.party_feedback = Some(tr!("Party changed; refresh party accounting"));
        }
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

#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn bevy_party_selected(&self, selected: bool, chat: bool, name: &str) {
        self.party_initialize(selected, chat);
        lock(&self.0).party.own_character_name = name.to_owned();
    }
    pub(crate) fn bevy_party_response(&self, response: &PartyResponse) {
        self.party_result(response);
    }
}
