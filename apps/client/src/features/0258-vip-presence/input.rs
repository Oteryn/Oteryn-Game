use crate::play::{PlayLink, mailbox::lock};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn vip_presence_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::vip_presence::open_key(events) {
            return false;
        }
        lock(&self.mailbox).vip_presence.open = true;
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        true
    }
}
