#[path = "input/selector.rs"]
pub(in crate::play) mod selector;
#[path = "input/catalogue.rs"]
pub(in crate::play) mod catalogue;
#[path = "input/charm_browser.rs"]
pub(in crate::play) mod charm_browser;
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::charm_progression::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.charm_progression;
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
            if oteryn_client_ui::charm_progression::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn charm_progression_edit(&self, field: usize, value: String) {
        edit(
            &mut lock(&self.mailbox).properties.charm_progression,
            field,
            value,
        );
    }
    pub fn charm_progression_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn charm_progression_open(&self) {
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).properties.charm_progression.open = true;
        }
    }
    pub fn charm_progression_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::charm_progression::open_key(events) {
            return false;
        }
        self.charm_progression_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::charm_progression::Panel,
    field: usize,
    value: String,
) {
    if panel.pending || value.len() > 7996 || value.contains('\0') {
        return;
    }
    let value = oteryn_client_ui::charm_progression::charm_browser::resolve(panel, field, &value)
        .unwrap_or(value);
    if let Some(slot) = panel.fields.get_mut(field) {
        if *slot == value {
            return;
        }
        *slot = value;
    }
}
