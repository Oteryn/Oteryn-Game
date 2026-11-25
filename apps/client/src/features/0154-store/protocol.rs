use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn store_modal_answered(
        &self,
        answer: oteryn_client_runtime::server_modal::Answer,
    ) {
        let mut mailbox = lock(&self.0);
        if mailbox.store.hireling_modal_id == Some(answer.dialog_id) {
            mailbox.store.hireling_modal_id = None;
            mailbox.store.confirmed_purchase = None;
            mailbox.store.feedback = None;
        }
    }
    pub(in crate::play) fn store_initialize(&self, selected: bool, hireling_modal_selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.store.selected = selected;
        mailbox.store_inbox_query_queued = false;
        mailbox.store.inbox = Default::default();
        mailbox.store.inbox_page_cursor = None;
        mailbox.store.inbox_query_pending = None;
        mailbox.store.claim = None;
        mailbox.store.cosmetic_tokens = None;
        mailbox.store.token_supported = false;
        mailbox.store.token_balance_refresh = false;
        mailbox.store.hireling_modal_selected = hireling_modal_selected;
        mailbox.store.gift_open = false;
        mailbox.store.gift_fields = Default::default();
        mailbox.store.gift_reviewed = None;
        mailbox.store.gift = Default::default();
    }
    pub(in crate::play) fn store_tokens_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.store.token_supported = selected && mailbox.store.selected;
        mailbox.store.token_balance_refresh = mailbox.store.token_supported;
    }
    pub(in crate::play) fn store_lamp_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        use oteryn_client_runtime::world_interaction::world_object as wire;
        let mut mailbox = lock(&self.0);
        mailbox.store.pending = false;
        use oteryn_client_runtime::server_mechanic::CommandStatus;
        if matches!(response.status, CommandStatus::Accepted | CommandStatus::DuplicateReplay)
            && matches!(wire::decode_use_result(&response.payload), Ok(wire::UseDisposition::Committed))
        {
            mailbox.store.inbox.lamp_definition = None;
            mailbox.store_inbox_query_queued = mailbox.cyclopedia_character.items.open;
        } else {
            mailbox.store.feedback = Some("The Hireling could not be placed here. Stand on an interior tile in your House.".into());
        }
    }

    pub(in crate::play) fn store_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        let mut mailbox = lock(&self.0);
        let action = mailbox.store.pending_action;
        let inbox_query = mailbox.store.inbox_query_pending.is_some();
        oteryn_client_ui::store::apply_result(&mut mailbox.store, response);
        let text = if let Some(feedback) = &mailbox.store.feedback {
            feedback.clone()
        } else if matches!(action, 185 | 6188)
            && let Some(purchase) = &mailbox.store.purchase
        {
            format!("Purchase: {:?}", purchase.code)
        } else if action == 186 && !inbox_query
            && let Some(claim) = &mailbox.store.claim
        {
            format!("Inbox claim: {:?}", claim.result)
        } else if action == 183 {
            "Store catalogue updated.".into()
        } else if action == 184 {
            "Store purchase history updated.".into()
        } else {
            return;
        };
        let received = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        #[cfg(bevy_shell)]
        mailbox.window_notice(crate::play::feature_local_text_messages::view::NoticeKind::Store, text.clone());
        mailbox.chat.console.notices.push(
            oteryn_client_ui::chat::NoticeClass::Store,
            &text,
            received,
        );
    }
}

impl Outbox {
    pub(in crate::play) fn coin_gift_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::store::gift_result(&mut lock(&self.0).store, response);
    }
}
