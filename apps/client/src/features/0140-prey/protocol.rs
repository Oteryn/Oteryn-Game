use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::prey::PreyResponse;

impl Outbox {
    pub(in crate::play) fn prey_initialize(&self, selected: bool) {
        lock(&self.0).prey.selected = selected;
    }

    pub(in crate::play) fn prey_result(&self, response: &PreyResponse) {
        oteryn_client_ui::prey::apply_result(&mut lock(&self.0).prey, response);
    }
}
