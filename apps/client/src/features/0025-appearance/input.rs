use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::appearance::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.appearance;
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
            if oteryn_client_ui::appearance::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn appearance_colour(&self, part: usize) -> bool {
        lock(&self.mailbox).properties.appearance.cycle_colour(part)
    }
    pub fn appearance_addon(&self, bit: u8) -> bool {
        lock(&self.mailbox).properties.appearance.toggle_addon(bit)
    }
    pub fn appearance_edit(&self, field: usize, value: String) {
        edit(&mut lock(&self.mailbox).properties.appearance, field, value);
    }
    #[cfg(bevy_shell)]
    pub(crate) fn appearance_preset(&self, expected: &oteryn_client_ui::appearance::Panel, fields: &[String; 4]) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.properties.appearance;
        if panel != expected || !panel.open || !panel.selected || panel.pending { return false; }
        panel.fields[..4].clone_from_slice(fields);
        true
    }
    #[cfg(bevy_shell)]
    pub(crate) fn appearance_accept_preset(&self, riding: bool) -> Option<bool> {
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.properties.appearance.open || mailbox.properties.appearance.pending { return None; }
        submit(&mut mailbox, &self.commands, Intent::Apply);
        let panel = &mailbox.properties.appearance;
        panel.pending.then_some(riding && !panel.fields[2].is_empty())
    }
    pub fn appearance_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn appearance_open(&self) {
        lock(&self.mailbox).properties.appearance.open = true;
    }
    pub fn appearance_close(&self) {
        lock(&self.mailbox).properties.appearance.open = false;
    }
    pub fn appearance_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::appearance::open_key(events) {
            return false;
        }
        self.appearance_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::appearance::Panel,
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
