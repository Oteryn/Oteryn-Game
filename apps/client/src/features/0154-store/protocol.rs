use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn store_initialize(&self, selected: bool) {
        lock(&self.0).store.selected = selected;
    }
    pub(in crate::play) fn store_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::store::apply_result(&mut lock(&self.0).store, response);
    }
}
