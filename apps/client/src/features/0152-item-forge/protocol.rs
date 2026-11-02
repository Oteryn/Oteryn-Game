use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn item_forge_initialize(&self, selected: bool) {
        lock(&self.0).item_forge.selected = selected;
    }
    pub(in crate::play) fn item_forge_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::item_forge::apply_result(&mut lock(&self.0).item_forge, response);
    }
}
