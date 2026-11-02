use crate::play::{PlayLink, mailbox::lock};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn chat_party_room_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::chat_party_room::open_key(events) {
            return false;
        }
        lock(&self.mailbox).chat_party_room.open = true;
        #[cfg(windows)]
        super::view::open_window(self);
        true
    }
}
