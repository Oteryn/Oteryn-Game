//! Bevy-only field-bound access to the retained Guild Bank owner.
use crate::play::mailbox::Mailbox;
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::lock},
};
use oteryn_client_ui::{
    guild::guild::GuildSummary,
    guild_bank::{Intent, Panel},
};
use std::sync::{Arc, Mutex, Weak};
#[derive(Clone, Debug)]
pub(crate) struct GuildBankView {
    pub panel: Panel,
    membership: Option<GuildSummary>,
    session: Weak<Mutex<Mailbox>>,
}
impl PartialEq for GuildBankView {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
            && self.panel == other.panel
            && self.membership == other.membership
    }
}
#[derive(Clone)]
pub(crate) enum GuildBankControl {
    Close,
    Request { intent: Intent, fields: [String; 2] },
}
impl PlayLink {
    pub(crate) fn guild_bank_window_view(&self) -> GuildBankView {
        let mailbox = lock(&self.mailbox);
        GuildBankView {
            panel: mailbox.properties.guild_bank.clone(),
            membership: mailbox.guild.own.clone(),
            session: Arc::downgrade(&self.mailbox),
        }
    }
    pub(crate) fn guild_bank_window_open(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !available(&mailbox)
            || self.commands.is_closed()
            || mailbox.properties.guild_bank.pending
        {
            return false;
        }
        if mailbox.properties.guild_bank.fields[0].is_empty()
            && let Some(own) = &mailbox.guild.own
        {
            mailbox.properties.guild_bank.fields[0] =
                own.guild.iter().map(|byte| format!("{byte:02x}")).collect();
        }
        mailbox.properties.guild_bank.open = true;
        true
    }
    pub(crate) fn guild_bank_window_dispatch(
        &self,
        expected: &GuildBankView,
        control: GuildBankControl,
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected.session.ptr_eq(&Arc::downgrade(&self.mailbox))
            || !available(&mailbox)
            || self.commands.is_closed()
            || mailbox.properties.guild_bank != expected.panel
            || mailbox.guild.own != expected.membership
            || !expected.panel.open
        {
            return false;
        }
        let GuildBankControl::Request { intent, fields } = control else {
            mailbox.properties.guild_bank.open = false;
            return true;
        };
        if !expected.panel.selected
            || expected.panel.pending
            || matches!(intent, Intent::Deposit | Intent::Withdraw | Intent::Claim) && !confirmed
        {
            return false;
        }
        // Prepare all fields and the canonical command on a copy before touching the owner.
        let mut candidate = expected.panel.clone();
        for (field, value) in fields.into_iter().enumerate() {
            if value.len() > 7996 || value.contains('\0') {
                return false;
            }
            super::super::input::edit(&mut candidate, field, value);
        }
        let Ok(command) = super::super::protocol::command(&candidate, intent) else {
            return false;
        };
        if !oteryn_client_ui::guild_bank::request(&mut candidate, command.kind) {
            return false;
        }
        if self.commands.send(PlayCommand::Feature(command)).is_err() {
            return false;
        }
        mailbox.properties.guild_bank = candidate;
        true
    }
}
fn available(mailbox: &Mailbox) -> bool {
    mailbox.shared_frontend
        && !mailbox.inventory_shutdown
        && !mailbox.world_input_blocked()
        && !mailbox.server_modal.is_active()
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_guild_bank_panel(&self, panel: Panel) {
        lock(&self.0).properties.guild_bank = panel;
    }
    pub(crate) fn bevy_guild_bank_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        reply: &oteryn_session::property_services::guild_bank::GuildBankReply,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = oteryn_session::property_services::guild_bank::encode_guild_bank_reply(reply)
            .map_err(|e| format!("{e:?}"))?;
        self.properties_result(
            command,
            &oteryn_client_runtime::raw_feature::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload,
            },
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "bound_tests.rs"]
mod tests;
