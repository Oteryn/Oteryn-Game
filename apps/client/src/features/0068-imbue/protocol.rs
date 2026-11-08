use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::imbue::ImbueResponse;

impl Outbox {
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
