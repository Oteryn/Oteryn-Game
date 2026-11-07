use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::hireling::HirelingResponse;

impl Outbox {
    pub(in crate::play) fn hireling_initialize(&self, selected: bool) {
        lock(&self.0).hireling.selected = selected;
    }

    pub(in crate::play) fn hireling_result(&self, response: &HirelingResponse) {
        oteryn_client_ui::hireling::apply_result(&mut lock(&self.0).hireling, response);
    }
}
