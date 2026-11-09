use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::imbue::ImbueResponse;

impl Outbox {
    pub(in crate::play) fn imbue_items_changed(&self) {
        let mut mailbox = lock(&self.0);
        if let Some(item) = mailbox.imbue.item
            && !super::input::carried_items(&mailbox)
                .iter()
                .any(|entry| entry.handle == item)
        {
            mailbox.imbue.invalidate_item_scope();
        }
    }

    pub(in crate::play) fn imbue_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        let details_selected = mailbox.imbue.details_selected;
        mailbox.imbue = oteryn_client_ui::imbue::ImbuePanel {
            selected,
            details_selected,
            ..Default::default()
        };
        mailbox.imbue_shutdown = false;
    }

    pub(in crate::play) fn imbue_result(&self, response: &ImbueResponse) {
        oteryn_client_ui::imbue::apply_result(&mut lock(&self.0).imbue, response);
    }
}

pub(in crate::play) fn equipment_inspection_result(
    mailbox: &mut crate::play::mailbox::Mailbox,
    response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
) {
    let scope = super::input::tracker::scope(mailbox);
    mailbox.imbue.equipment.reconcile(scope);
    if mailbox.world_input_blocked() {
        mailbox.imbue.equipment.pending = None;
        return;
    }
    if matches!(mailbox.discovery.inspection.last_request,
        Some(oteryn_client_ui::inspection::Request::Item(item)) if mailbox.imbue.equipment.pending == Some(item))
    {
        mailbox.imbue.equipment.accept(response);
    }
}
