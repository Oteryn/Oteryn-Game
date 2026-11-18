use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn hotkey_equip_items(&self, mut items: Vec<u32>) {
        items.sort_unstable();
        items.dedup();
        lock(&self.0).hotkey_equip.items = items;
    }
    pub(in crate::play) fn hotkey_equip_initialize(&self, selected: bool) {
        lock(&self.0).hotkey_equip.selected = selected;
    }
    pub(in crate::play) fn hotkey_equip_tiered(&self, tiered: bool) {
        lock(&self.0).hotkey_equip.tiered = tiered;
    }
    pub(in crate::play) fn hotkey_equip_smart_mode(&self, active: bool) {
        lock(&self.0).hotkey_equip.smart_active = active;
    }
    pub(in crate::play) fn hotkey_equip_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::hotkey_equip::apply_result(&mut lock(&self.0).hotkey_equip, response);
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn bevy_equip_capabilities(&self, tiered: bool) {
        self.hotkey_equip_initialize(true);
        self.hotkey_equip_tiered(tiered);
    }

    /// The equippable carried definitions the server reports, for the Bevy action editor.
    pub(crate) fn bevy_hotkey_equip_items(&self, items: Vec<u32>) {
        self.hotkey_equip_items(items);
    }
}
