#[path = "controller.rs"]
pub(in crate::play) mod controller;
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn item_forge_request(&self, payload: Vec<u8>) {
        let mut mailbox = lock(&self.mailbox);
        let command = oteryn_client_ui::item_forge::command(payload);
        if let Some(command) =
            oteryn_client_ui::item_forge::request(&mut mailbox.item_forge, command)
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.item_forge.pending = false;
            mailbox.item_forge.feedback = Some("The session has ended".into());
        }
    }
    pub fn item_forge_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::item_forge::open_key(events) {
            return false;
        }
        {
            let mut mailbox = lock(&self.mailbox);
            if !mailbox.item_forge.operation.requires_pair() && mailbox.item_forge.retry.is_none() {
                mailbox.item_forge.operation =
                    oteryn_client_runtime::cui3::item_forge::Operation::Fusion;
            }
            controller::activate(&mut mailbox, &self.commands, controller::Action::Refresh);
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).item_forge.open = true;
        }
        true
    }
}
