use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::house_transfer::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.house_transfer;
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
            if oteryn_client_ui::house_transfer::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn house_transfer_edit(&self, field: usize, value: String) {
        edit(
            &mut lock(&self.mailbox).properties.house_transfer,
            field,
            value,
        );
    }
    pub fn house_transfer_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn house_transfer_open(&self) {
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).properties.house_transfer.open = true;
        }
    }
    pub fn house_transfer_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::house_transfer::open_key(events) {
            return false;
        }
        self.house_transfer_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::house_transfer::Panel,
    field: usize,
    value: String,
) {
    if panel.pending || value.len() > 512 || value.contains('\0') {
        return;
    }
    if let Some(slot) = panel.fields.get_mut(field) {
        if *slot == value {
            return;
        }
        *slot = value;
    }
}
