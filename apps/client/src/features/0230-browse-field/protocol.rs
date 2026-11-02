use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn browse_field_initialize(&self, selected: bool) {
        lock(&self.0).browse_field.selected = selected;
    }
    pub(in crate::play) fn browse_field_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::browse_field::apply_result(&mut lock(&self.0).browse_field, response);
    }
}
