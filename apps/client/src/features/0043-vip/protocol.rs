use crate::play::{Outbox, mailbox::lock};

impl Outbox {
    pub(in crate::play) fn vip_initialize(&self, selected: bool) {
        lock(&self.0).vip.selected = selected;
    }
    pub(in crate::play) fn vip_result(&self, response: &oteryn_client_runtime::vip::VipResponse) {
        oteryn_client_ui::vip::apply_result(&mut lock(&self.0).vip, response);
    }
}
