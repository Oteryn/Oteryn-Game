use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::character_info::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.character_info;
    if !panel.selected {
        panel.feedback = "Unavailable in this session".into();
        return;
    }
    if panel.pending {
        panel.feedback = "Waiting for the server".into();
        return;
    }
    match super::protocol::command(panel, intent) {
        Ok(command) => {
            if oteryn_client_ui::character_info::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed("The session has ended");
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn character_info_edit(&self, field: usize, value: String) {
        edit(
            &mut lock(&self.mailbox).properties.character_info,
            field,
            value,
        );
    }
    pub fn character_info_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn character_info_open(&self) {
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).properties.character_info.open = true;
        }
    }
    pub fn character_info_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::character_info::open_key(events) {
            return false;
        }
        self.character_info_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::character_info::Panel,
    field: usize,
    value: String,
) {
    if panel.pending || value.len() > 7996 || value.contains('\0') {
        return;
    }
    if let Some(slot) = panel.fields.get_mut(field) {
        if *slot == value {
            return;
        }
        *slot = value;
    }
}
