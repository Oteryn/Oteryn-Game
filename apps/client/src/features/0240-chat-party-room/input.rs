use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn chat_party_room_text(&self, text: String) {
        lock(&self.mailbox).chat_party_room.text = text;
    }

    pub fn chat_party_room_send(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(intent) =
            oteryn_client_ui::chat_party_room::request(&mut mailbox.chat_party_room)
            && self.commands.send(PlayCommand::ChatPublic(intent)).is_err()
        {
            mailbox.chat_party_room.pending = false;
            mailbox.chat_party_room.feedback = Some(tr!("The session has ended").into());
        }
    }

    pub fn chat_party_room_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::chat_party_room::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).chat_party_room.open = true;
        }
        true
    }
}
