use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::imbue::ImbueResponse;

impl Outbox {
    pub(in crate::play) fn imbue_initialize(&self, selected: bool) {
        lock(&self.0).imbue.selected = selected;
    }

    pub(in crate::play) fn imbue_result(&self, response: &ImbueResponse) {
        oteryn_client_ui::imbue::apply_result(&mut lock(&self.0).imbue, response);
    }
}
