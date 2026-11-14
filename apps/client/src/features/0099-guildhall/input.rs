use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::guildhall::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.guildhall;
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
            if oteryn_client_ui::guildhall::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn guildhall_edit(&self, field: usize, value: String) {
        edit(&mut lock(&self.mailbox).properties.guildhall, field, value);
    }
    pub fn guildhall_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn guildhall_open(&self) {
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).properties.guildhall.open = true;
        }
    }
    pub fn guildhall_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::guildhall::open_key(events) {
            return false;
        }
        self.guildhall_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::guildhall::Panel,
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
