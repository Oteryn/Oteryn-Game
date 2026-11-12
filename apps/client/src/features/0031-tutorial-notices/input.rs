use crate::play::PlayLink;
#[cfg(not(windows))]
use crate::play::mailbox::lock;
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn notices_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::tutorial_notices::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).notices.open = true;
        }
        true
    }
}
