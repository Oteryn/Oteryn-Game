use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::player_report::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.player_report;
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
            if oteryn_client_ui::player_report::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn player_report_edit(&self, field: usize, value: String) {
        edit(
            &mut lock(&self.mailbox).properties.player_report,
            field,
            value,
        );
    }
    pub fn player_report_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn player_report_open(&self) {
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).properties.player_report.open = true;
        }
    }
    pub fn player_report_input(&self, events: &[NormalizedInputEvent]) -> bool {
        use oteryn_input_actions::{ButtonState, Modifier, Modifiers};
        if events.iter().any(|event| matches!(event, NormalizedInputEvent::Key {
            code, state: ButtonState::Pressed, modifiers, repeat: false
        } if code.get() == 69 && *modifiers == Modifiers::one(Modifier::Control).with(Modifier::Shift))) {
            self.bug_report_open();
            return true;
        }
        if !oteryn_client_ui::player_report::open_key(events) {
            return false;
        }
        self.player_report_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::player_report::Panel,
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
