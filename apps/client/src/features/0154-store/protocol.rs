use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn store_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.store.selected = selected;
        mailbox.store.gift_open = false;
        mailbox.store.gift_fields = Default::default();
        mailbox.store.gift_reviewed = None;
    }
    pub(in crate::play) fn store_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::store::apply_result(&mut lock(&self.0).store, response);
    }
}
