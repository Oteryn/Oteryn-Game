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

#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn bevy_prey_initialize(&self, selected: bool) {
        self.prey_initialize(selected);
    }
    pub(crate) fn bevy_prey_result(&self, response: &PreyResponse) {
        self.prey_result(response);
    }
}
