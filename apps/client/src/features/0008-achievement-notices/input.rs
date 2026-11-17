use crate::play::PlayLink;
use crate::play::mailbox::lock;
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn achievement_notices_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::achievement_notices::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).achievement_notices.open = true;
        }
        true
    }
}
