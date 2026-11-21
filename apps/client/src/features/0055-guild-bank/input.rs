use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::guild_bank::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.guild_bank;
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
            if oteryn_client_ui::guild_bank::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed(tr!("The session has ended"));
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    pub fn guild_bank_edit(&self, field: usize, value: String) {
        edit(&mut lock(&self.mailbox).properties.guild_bank, field, value);
    }
    pub fn guild_bank_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn guild_bank_open(&self) {
        {
            let mut mailbox = lock(&self.mailbox);
            if mailbox.properties.guild_bank.fields[0].is_empty()
                && let Some(own) = &mailbox.guild.own
            {
                let id: String = own.guild.iter().map(|byte| format!("{byte:02x}")).collect();
                mailbox.properties.guild_bank.fields[0] = id;
            }
        }
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).properties.guild_bank.open = true;
        }
    }
    pub fn guild_bank_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::guild_bank::open_key(events) {
            return false;
        }
        self.guild_bank_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::guild_bank::Panel,
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
        if field == 0 {
            panel.summary.clear();
            panel.next = None;
            panel.next_claim = None;
            panel.loaded_guild.clear();
        }
    }
}
