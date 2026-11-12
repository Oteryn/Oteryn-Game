use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::house_furniture_wrap::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.house_furniture_wrap;
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
            if oteryn_client_ui::house_furniture_wrap::request(panel, command.payload.clone())
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
    pub fn house_furniture_edit(&self, value: String) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.properties.house_furniture_wrap;
        if !panel.pending && value.len() <= 3 && value.chars().all(|ch| ch.is_ascii_digit()) {
            panel.choice = value;
        }
    }
    pub fn house_furniture_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn house_furniture_open(&self) {
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).properties.house_furniture_wrap.open = true;
        }
        self.house_furniture_request(Intent::Refresh);
    }
    pub fn house_furniture_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::house_furniture_wrap::open_key(events) {
            return false;
        }
        self.house_furniture_open();
        true
    }
}
