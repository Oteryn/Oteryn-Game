#[path = "input/atlas.rs"]
pub(in crate::play) mod atlas;
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::cyclopedia_map::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.cyclopedia_map;
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
            if oteryn_client_ui::cyclopedia_map::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn cyclopedia_map_edit(&self, field: usize, value: String) {
        edit(
            &mut lock(&self.mailbox).properties.cyclopedia_map,
            field,
            value,
        );
    }
    pub fn cyclopedia_map_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn cyclopedia_map_open(&self) {
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).properties.cyclopedia_map.open = true;
        }
    }
    pub fn cyclopedia_map_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::cyclopedia_map::open_key(events) {
            return false;
        }
        self.cyclopedia_map_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::cyclopedia_map::Panel,
    field: usize,
    value: String,
) {
    if panel.pending || value.len() > 64 || value.contains('\0') {
        return;
    }
    if let Some(slot) = panel.fields.get_mut(field) {
        if *slot == value {
            return;
        }
        *slot = value;
    }
}

#[cfg(test)]
#[path = "input/tests/atlas.rs"]
mod atlas_tests;
