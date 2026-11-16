use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::party_skull::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.party_skull;
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
            if oteryn_client_ui::party_skull::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn party_skull_edit(&self, field: usize, value: String) {
        edit(
            &mut lock(&self.mailbox).properties.party_skull,
            field,
            value,
        );
    }
    pub fn party_skull_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn party_skull_close(&self) {
        lock(&self.mailbox).properties.party_skull.open = false;
    }
    pub fn party_skull_open(&self) {
        lock(&self.mailbox).properties.party_skull.open = true;
    }
    pub fn party_skull_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::party_skull::open_key(events) {
            return false;
        }
        self.party_skull_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::party_skull::Panel,
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

#[cfg(all(test, bevy_shell))]
impl crate::play::Outbox {
    pub(crate) fn bevy_pvp_initialize(&self) {
        lock(&self.0).properties.party_skull.selected = true;
    }
    pub(crate) fn bevy_pvp_result(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        super::protocol::apply(&mut lock(&self.0).properties.party_skull, command, result);
    }
}
