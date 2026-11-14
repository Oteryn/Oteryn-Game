use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::mail::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.mail;
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
            if oteryn_client_ui::mail::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn mail_edit(&self, field: usize, value: String) {
        edit(&mut lock(&self.mailbox).properties.mail, field, value);
    }
    pub fn mail_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn mail_open(&self) {
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).properties.mail.open = true;
        }
    }
    pub fn mail_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if self.staff_input(events) {
            return true;
        }
        if !oteryn_client_ui::mail::open_key(events) {
            return false;
        }
        {
            let mut mailbox = lock(&self.mailbox);
            if !mailbox.properties.mail.pending
                && let Some(handle) = mailbox.inventory.selected_item().map(|item| item.handle)
            {
                mailbox.properties.mail.fields[0] = handle.get().to_string();
            }
        }
        self.mail_open();
        true
    }
}

#[cfg(windows)]
#[path = "context.rs"]
mod context;

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::mail::Panel,
    field: usize,
    value: String,
) {
    if panel.pending
        || !oteryn_client_ui::modal_dialogs::TextEditor::accepts(
            &value,
            super::protocol::MAX_TEXT_BYTES,
        )
    {
        return;
    }
    if let Some(slot) = panel.fields.get_mut(field) {
        if *slot == value {
            return;
        }
        *slot = value;
    }
}
