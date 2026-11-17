use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::house_furniture_rotation::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.house_furniture_rotation;
    if !panel.selected {
        panel.feedback = tr!("Unavailable in this session").into();
        return;
    }
    if panel.pending {
        panel.feedback = tr!("Waiting for the server").into();
        return;
    }
    match super::protocol::command(panel, intent) {
        Ok(command) => {
            if oteryn_client_ui::house_furniture_rotation::request(panel, command.payload.clone())
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.pending = false;
                panel.feedback = tr!("The session has ended").into();
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn house_rotation_edit(&self, value: String) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.properties.house_furniture_rotation;
        if !panel.pending && value.len() <= 3 && value.chars().all(|ch| ch.is_ascii_digit()) {
            panel.choice = value;
        }
    }
    pub fn house_rotation_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn house_rotation_open(&self) {
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).properties.house_furniture_rotation.open = true;
        }
        self.house_rotation_request(Intent::Refresh);
    }
    pub fn house_rotation_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::house_furniture_rotation::open_key(events) {
            return false;
        }
        self.house_rotation_open();
        true
    }
}
