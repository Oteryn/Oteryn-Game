use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn depot_search_request(&self, payload: Vec<u8>) {
        let mut mailbox = lock(&self.mailbox);
        let command = oteryn_client_ui::depot_search::command(payload);
        if let Some(command) =
            oteryn_client_ui::depot_search::request(&mut mailbox.depot_search, command)
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.depot_search.pending = false;
            mailbox.depot_search.feedback = Some(tr!("The session has ended").into());
        }
    }
    pub fn depot_search_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        if !oteryn_client_ui::depot_search::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).depot_search.open = true;
        }
        true
    }
}
