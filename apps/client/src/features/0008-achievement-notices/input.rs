use crate::play::PlayLink;
#[cfg(not(windows))]
use crate::play::mailbox::lock;
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn achievement_notices_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::achievement_notices::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).achievement_notices.open = true;
        }
        true
    }
}
