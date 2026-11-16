use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::item_inscription::{self as inscription, Known, WriteCheck};
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
    // Validate every submitted field before producing any command, including unused fields.
    if panel
        .fields
        .iter()
        .any(|value| !oteryn_client_ui::modal_dialogs::TextEditor::accepts(value, 7996)) {
        panel.feedback = "A form field exceeds its UTF-8 byte limit or contains invalid text".into();
        return;
    }
    if intent == Intent::Write
        && let Ok(handle) = super::protocol::handle(&panel.fields[0])
    {
        match inscription::write_check(&mailbox.properties.mail_inscription, handle.get(), &panel.fields[2]) {
            WriteCheck::Allowed => {}
            WriteCheck::Read => {
                read(&mut mailbox.properties, commands);
                return;
            }
            WriteCheck::Refused(message) => {
                panel.feedback = message.into();
                return;
            }
        }
    }
    let panel = &mut mailbox.properties.mail;
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
        lock(&self.mailbox).properties.mail.open = true;
    }
    /// Opens the writing window for one item, as Tibia Global's Use on a writable item.
    pub fn mail_write_item(&self, handle: oteryn_session::ItemHandle) {
        let mut mailbox = lock(&self.mailbox);
        let properties = &mut mailbox.properties;
        if properties.mail.pending {
            // The in-flight request keeps its item; the reply retargets the form to this one.
            properties.mail_deferred = Some(handle);
        } else {
            properties.mail_deferred = None;
            edit(&mut properties.mail, 0, handle.get().to_string());
            read(properties, &self.commands);
        }
        properties.mail.open = true;
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
                edit(&mut mailbox.properties.mail, 0, handle.get().to_string());
                read(&mut mailbox.properties, &self.commands);
            }
        }
        self.mail_open();
        true
    }
}

/// Asks for the current text of the writing window's item. The window waits for the reply,
/// which fills the text field, so a Write never replaces a text the player has not seen.
fn read(properties: &mut super::protocol::Panels, commands: &UnboundedSender<PlayCommand>) {
    let Ok(handle) = super::protocol::handle(&properties.mail.fields[0]) else {
        return;
    };
    let Some(command) = inscription::request(&mut properties.mail_inscription, handle.get())
    else {
        return;
    };
    if oteryn_client_ui::mail::request(&mut properties.mail, command.kind) {
        properties.mail.feedback = tr!("Reading the item's text").into();
        if commands.send(PlayCommand::Feature(command)).is_err() {
            properties.mail.failed(tr!("The session has ended"));
        }
    }
}

/// While the writing window is open and idle, reads the text of an item it was pointed at
/// while another request was pending.
pub(in crate::play) fn tick(mailbox: &mut Mailbox, commands: &UnboundedSender<PlayCommand>) {
    let properties = &mut mailbox.properties;
    if properties.mail.pending || !properties.mail_inscription.selected {
        return;
    }
    let Ok(handle) = super::protocol::handle(&properties.mail.fields[0]) else {
        return;
    };
    let handle = handle.get();
    let settled = match properties.mail_inscription.known {
        Known::Read { handle: read, .. } | Known::Refused(read) => read == handle,
        Known::Unread | Known::Reading(_) => false,
    };
    if !settled {
        read(properties, commands);
    }
}

/// The read's reply: the server's text fills the editor; a refusal leaves it blocked.
pub(in crate::play) fn inscription_result(
    properties: &mut super::protocol::Panels,
    command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
) {
    let panel = &mut properties.mail;
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;
    let reply = inscription::apply_result(&mut properties.mail_inscription, command, result);
    panel.feedback = match reply {
        Some(reply) => {
            panel.fields[2] = reply.text;
            if reply.writable {
                String::new()
            } else {
                tr!("This item cannot be written on").into()
            }
        }
        None => tr!("The server refused to read the item's text").into(),
    };
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::mail::Panel,
    field: usize,
    value: String,
) {
    if panel.pending {
        return;
    }
    if field == 0 && panel.fields[0] != value {
        panel.fields[2].clear();
    }
    // Retain rejected input so a later submit cannot send the previous stored value.
    if let Some(slot) = panel.fields.get_mut(field) {
        if *slot == value {
            return;
        }
        *slot = value;
    }
}
