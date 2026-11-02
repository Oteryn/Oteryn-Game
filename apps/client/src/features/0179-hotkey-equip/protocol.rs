use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn hotkey_equip_initialize(&self, selected: bool) {
        lock(&self.0).hotkey_equip.selected = selected;
    }
    pub(in crate::play) fn hotkey_equip_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::hotkey_equip::apply_result(&mut lock(&self.0).hotkey_equip, response);
    }
}
