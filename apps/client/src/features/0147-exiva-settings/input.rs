use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn exiva_request(&self, payload: Vec<u8>) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(payload) =
            oteryn_client_ui::exiva_settings::request(&mut mailbox.exiva, payload)
            && self.commands.send(PlayCommand::Exiva(payload)).is_err()
        {
            mailbox.exiva.pending = false;
            mailbox.exiva.error = Some("Session ended");
        }
    }
    pub fn exiva_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::exiva_settings::open_key(events) {
            return false;
        }
        self.exiva_request(vec![5]);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).exiva.open = true;
        }
        true
    }
}
