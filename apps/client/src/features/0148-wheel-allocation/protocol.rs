use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::wheel_allocation::WheelResponse;

impl Outbox {
    pub(in crate::play) fn wheel_initialize(&self, selected: bool) {
        lock(&self.0).wheel.selected = selected;
    }

    pub(in crate::play) fn wheel_result(&self, response: &WheelResponse) {
        let mut m = lock(&self.0);
        if oteryn_client_ui::wheel_allocation::apply_result(&mut m.wheel, response) {
            m.wheel.diagram.reset_requested = false;
        }
    }
}
