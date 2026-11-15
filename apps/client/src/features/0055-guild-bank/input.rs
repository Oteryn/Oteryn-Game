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
    sync_membership(mailbox);
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
        sync_membership(&mut lock(&self.mailbox));
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

pub(in crate::play) fn sync_membership(mailbox: &mut Mailbox) {
    let current = mailbox
        .guild
        .own
        .as_ref()
        .map(|own| {
            own.guild
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .unwrap_or_default();
    let panel = &mut mailbox.properties.guild_bank;
    if current == panel.automatic_guild {
        return;
    }
    if panel.fields[0].is_empty() || panel.fields[0] == panel.automatic_guild {
        panel.fields[0] = current.clone();
    }
    panel.automatic_guild = current;
    panel.summary.clear();
    panel.loaded_guild.clear();
    panel.next = None;
    panel.next_claim = None;
    panel.pending = false;
}

#[cfg(test)]
mod membership_tests {
    use super::*;
    #[test]
    fn automatic_guild_bank_target_and_balance_follow_membership() {
        let (link, _, _) = crate::play::play_channel();
        let summary = |guild| oteryn_client_runtime::guild::guild::GuildSummary {
            guild,
            name: "Guild".into(),
            state: 1,
            revision: 1,
            own_rank: 0,
            title: String::new(),
            message: String::new(),
            ranks: vec![],
        };
        lock(&link.mailbox).guild.own = Some(summary([1; 16]));
        link.guild_bank_open();
        assert_eq!(link.guild_bank_panel().fields[0], "01".repeat(16));
        lock(&link.mailbox).properties.guild_bank.summary = "Guild A balance: 100".into();
        lock(&link.mailbox).guild.own = None;
        link.guild_bank_open();
        assert!(link.guild_bank_panel().fields[0].is_empty());
        assert!(link.guild_bank_panel().summary.is_empty());
        lock(&link.mailbox).guild.own = Some(summary([2; 16]));
        link.guild_bank_open();
        assert_eq!(link.guild_bank_panel().fields[0], "02".repeat(16));
        assert!(link.guild_bank_panel().summary.is_empty());
    }
}
