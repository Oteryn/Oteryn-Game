use crate::play::PlayLink;
use crate::play::mailbox::lock;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn notices_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::tutorial_notices::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).notices.open = true;
        }
        true
    }
}
