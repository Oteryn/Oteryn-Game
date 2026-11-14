use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    receipt: bool,
) {
    let panel = &mut mailbox.properties.staff;
    if !panel.selected {
        panel.feedback = tr!("Unavailable in this session").into();
        return;
    }
    if panel.pending {
        return;
    }
    match super::protocol::command(panel, receipt) {
        Ok(command) => {
            panel.pending = true;
            panel.pending_kind = command.kind;
            panel.feedback = tr!("Waiting for server authorization").into();
            if commands.send(PlayCommand::Feature(command)).is_err() {
                *panel = super::state::Panel {
                    feedback: tr!("The session has ended").into(),
                    ..Default::default()
                };
            }
        }
        Err(message) => panel.feedback = message.into(),
    }
}
impl PlayLink {
    pub fn staff_edit(&self, field: usize, value: String) {
        let mut mailbox = lock(&self.mailbox);
        edit(&mut mailbox.properties.staff, field, value);
    }
    pub fn staff_submit(&self, receipt: bool) {
        submit(&mut lock(&self.mailbox), &self.commands, receipt);
    }
    pub fn staff_open(&self) {
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).properties.staff.open = true;
        }
    }
    pub fn staff_input(&self, events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::staff_tools::open_key(events) {
            return false;
        }
        self.staff_open();
        true
    }
}
pub(super) fn edit(panel: &mut super::state::Panel, field: usize, value: String) {
    if panel.pending || value.len() > 8192 || value.contains('\0') {
        return;
    }
    if let Some(slot) = panel.fields.get_mut(field) {
        *slot = value;
    }
}
