use crate::play::{Outbox, mailbox::lock};
use std::time::Instant;
use oteryn_client_runtime::party::{CommandStatus, PartyResponse, party::COMMAND_TYPE_PARTY_QUERY};

impl Outbox {
    pub(in crate::play) fn party_initialize(&self, selected: bool, chat_selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.party_initialized = true;
        if selected {
            mailbox.party_notice_refresh.cancel_query();
        } else {
            mailbox.party_notice_refresh = Default::default();
        }
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
        mailbox.refresh_party_summons();
    }

    pub(in crate::play) fn party_result(&self, response: &PartyResponse) {
        let received_at = Instant::now();
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
        let query = mailbox
            .party
            .pending
            .as_ref()
            .is_some_and(|command| command.kind == COMMAND_TYPE_PARTY_QUERY);
        let feedback = mailbox.party.feedback;
        let applied = oteryn_client_ui::party_list::apply_result(&mut mailbox.party, response);
        if query {
            if mailbox.party_notice_refresh.automatic_pending() {
                mailbox.party.feedback = feedback;
            }
            mailbox.party_notice_refresh.finish_query(
                applied
                    && matches!(
                        response.status,
                        CommandStatus::Accepted | CommandStatus::DuplicateReplay
                    ),
                received_at,
            );
        }
        if applied {
            mailbox.refresh_party_summons();
        }
        let after = members(&mailbox.party);
        if let (Some(old), Some(new)) = (&before, &after)
            && old != new
            && mailbox.ui_sounds.len() < 32
        {
            mailbox.ui_sounds.push(crate::play::UiSound::Party);
        }
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
            let started = Instant::now();
            let result = session.party(request).await;
            if request.kind == COMMAND_TYPE_PARTY_QUERY
                && matches!(
                    result,
                    Err(oteryn_session::SessionError::ScopeChanged
                        | oteryn_session::SessionError::ActivityScopeChanged)
                )
            {
                // The session reads the handoff only after writing this query.
                lock(&self.0).party_notice_refresh.finish_query(false, started);
            }
            result?
        };
        self.party_result(&response);
        Ok(())
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn bevy_party_selected(&self, selected: bool, chat: bool, name: &str) {
        self.party_initialize(selected, chat);
        lock(&self.0).party.own_character_name = name.to_owned();
    }
    pub(crate) fn bevy_party_response(&self, response: &PartyResponse) {
        self.party_result(response);
    }
}
