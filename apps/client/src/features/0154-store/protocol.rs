use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn store_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.store.selected = selected;
        mailbox.store.gift_open = false;
        mailbox.store.gift_fields = Default::default();
        mailbox.store.gift_reviewed = None;
        mailbox.store.gift = Default::default();
    }
    pub(in crate::play) fn store_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        let mut mailbox = lock(&self.0);
        let action = mailbox.store.pending_action;
        oteryn_client_ui::store::apply_result(&mut mailbox.store, response);
        let text = if let Some(feedback) = &mailbox.store.feedback {
            feedback.clone()
        } else if action == 185
            && let Some(purchase) = &mailbox.store.purchase
        {
            format!("Purchase: {:?}", purchase.code)
        } else if action == 186
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
