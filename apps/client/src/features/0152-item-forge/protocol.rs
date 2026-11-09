use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn item_forge_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        let preview_selected = mailbox.item_forge.preview.selected;
        mailbox.item_forge = oteryn_client_ui::item_forge::ItemForgePanel {
            selected,
            ..Default::default()
        };
        mailbox.item_forge.preview.selected = preview_selected;
    }
    pub(in crate::play) fn item_forge_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        oteryn_client_ui::item_forge::apply_result(&mut lock(&self.0).item_forge, response);
    }
}

impl Outbox {
    pub(in crate::play) fn item_forge_publish(&self, session: &impl crate::play::Stepper) {
        let items = session.depot_items();
        let mut live = items
            .inventory
            .main_backpack
            .into_iter()
            .chain(items.inventory.entries)
            .chain(items.views)
            .collect::<Vec<_>>();
        live.sort_by_key(|item| item.handle);
        live.dedup_by_key(|item| item.handle);
        lock(&self.0).item_forge.observe(live);
    }
    pub(in crate::play) fn item_forge_container_ready(&self, handle: oteryn_session::ItemHandle) {
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.item_forge;
        if panel.preview.navigation == Some(handle.get()) {
            panel.preview.navigation = None;
            panel.preview.refresh = true;
        }
    }
}
