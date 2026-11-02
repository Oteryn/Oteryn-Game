use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};

impl PlayLink {
    pub fn store_request(&self, command_type: u32, payload: Vec<u8>) {
        let mut mailbox = lock(&self.mailbox);
        let Some(command) = oteryn_client_ui::store::command(command_type, payload) else {
            return;
        };
        if let Some(command) = oteryn_client_ui::store::request(&mut mailbox.store, command)
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.store.pending = false;
            mailbox.store.feedback = Some("The session has ended".into());
        }
    }

    pub fn store_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::store::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).store.open = true;
        }
        true
    }
}
