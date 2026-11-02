use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn hotkey_equip_request(&self, payload: Vec<u8>) {
        let mut mailbox = lock(&self.mailbox);
        let command = oteryn_client_ui::hotkey_equip::command(payload);
        if let Some(command) =
            oteryn_client_ui::hotkey_equip::request(&mut mailbox.hotkey_equip, command)
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.hotkey_equip.pending = false;
            mailbox.hotkey_equip.feedback = Some("The session has ended".into());
        }
    }
    pub fn hotkey_equip_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        if !oteryn_client_ui::hotkey_equip::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).hotkey_equip.open = true;
        }
        true
    }
}
