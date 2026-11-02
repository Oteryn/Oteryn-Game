use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn browse_field_request(&self, payload: Vec<u8>) {
        let mut mailbox = lock(&self.mailbox);
        let command = oteryn_client_ui::browse_field::command(payload);
        if let Some(command) =
            oteryn_client_ui::browse_field::request(&mut mailbox.browse_field, command)
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.browse_field.pending = false;
            mailbox.browse_field.feedback = Some("The session has ended".into());
        }
    }
    pub fn browse_field_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        if !oteryn_client_ui::browse_field::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).browse_field.open = true;
        }
        true
    }
}
