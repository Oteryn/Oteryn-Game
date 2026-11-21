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
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.item_forge;
        let pair = panel.submitted.is_some_and(|request| request.operation.requires_pair());
        oteryn_client_ui::item_forge::apply_result(panel, response);
        if pair && panel.retry.is_none() {
            panel.preview.items.clear();
            panel.preview.page = None;
            panel.preview.refresh = true;
            panel.tier_effects.clear();
        }
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

#[allow(dead_code, reason = "Native combo state is exercised in play tests")]
pub(in crate::play) fn retained_bag(selected: u64, entries: &[(u64, String)]) -> u64 {
    entries.iter().find(|entry| entry.0 == selected).map_or(0, |entry| entry.0)
}

#[allow(dead_code, reason = "Native dialog keys are exercised in play tests")]
pub(in crate::play) fn resource_cancel_key(key: usize) -> bool {
    key == 27
}
