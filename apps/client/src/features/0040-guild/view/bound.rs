//! Bevy-only atomic adapters; the native Guild owner and reducer remain unchanged.
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::guild::{GuildPanel, Intent, request};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone, Debug)]
pub(crate) struct GuildWindowView {
    pub panel: GuildPanel,
    session: Weak<Mutex<Mailbox>>,
}
impl PartialEq for GuildWindowView {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session) && self.panel == other.panel
    }
}
impl GuildWindowView {
    pub(crate) fn same_session(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
    }
}
#[derive(Clone)]
pub(crate) enum GuildWindowAction {
    Close,
    Request(Intent),
}

/// These are visibility hints from the server's rank/state, never mutation authority.
/// The dispatcher/server still makes the final eligibility decision.
pub(crate) fn permitted(panel: &GuildPanel, intent: &Intent) -> bool {
    if !panel.open || !panel.selected || panel.pending {
        return false;
    }
    match intent {
        Intent::RefreshOwn | Intent::RefreshInvitations => true,
        Intent::RefreshRoster => panel.own.is_some(),
        Intent::MoreRoster => panel.own.is_some() && panel.next.is_some(),
        Intent::Join(guild) => {
            panel.own.is_none() && panel.invitations.iter().any(|i| &i.guild == guild)
        }
        Intent::Found(_) => panel.own.is_none(),
        Intent::RevokeInvite(_) => false, // The owner has no outgoing-invitation projection to bind.
        Intent::Invite(_) | Intent::Leave | Intent::Kick(_) | Intent::SetRank { .. } => {
            let Some(own) = &panel.own else {
                return false;
            };
            if !matches!(own.state, 1 | 2) || own.own_rank == 0 {
                return false;
            }
            match intent {
                Intent::Invite(_) => own.own_rank <= 2,
                Intent::Leave => own.own_rank > 1,
                Intent::Kick(character) => {
                    own.own_rank <= 2
                        && panel
                            .roster
                            .iter()
                            .any(|m| m.character == *character && m.level > own.own_rank)
                }
                Intent::SetRank { character, level } => {
                    *level > own.own_rank
                        && usize::from(*level) <= own.ranks.len()
                        && panel.roster.iter().any(|m| {
                            m.character == *character && m.level > own.own_rank && m.level != *level
                        })
                }
                _ => false,
            }
        }
    }
}
pub(crate) fn needs_confirmation(intent: &Intent) -> bool {
    matches!(
        intent,
        Intent::Leave | Intent::Kick(_) | Intent::SetRank { .. }
    )
}
fn available(mailbox: &Mailbox) -> bool {
    mailbox.shared_frontend
        && !mailbox.inventory_shutdown
        && !mailbox.world_input_blocked()
        && !mailbox.server_modal.is_active()
}
impl PlayLink {
    /// Session-wide gate shared by the two guild windows, before staging local confirmation.
    pub(crate) fn guild_window_available(&self) -> bool {
        let mailbox = lock(&self.mailbox);
        available(&mailbox) && !self.commands.is_closed()
    }
    pub(crate) fn guild_window_view(&self) -> GuildWindowView {
        GuildWindowView {
            panel: lock(&self.mailbox).guild.clone(),
            session: Arc::downgrade(&self.mailbox),
        }
    }
    pub(crate) fn guild_window_open(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !available(&mailbox) || self.commands.is_closed() || mailbox.guild.pending {
            return false;
        }
        let mut candidate = mailbox.guild.clone();
        candidate.open = true;
        if let Some(command) = request(&mut candidate, Intent::RefreshOwn)
            && self.commands.send(PlayCommand::Guild(command)).is_err()
        {
            return false;
        }
        mailbox.guild = candidate;
        true
    }
    pub(crate) fn guild_window_dispatch(
        &self,
        expected: &GuildWindowView,
        action: GuildWindowAction,
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected.session.ptr_eq(&Arc::downgrade(&self.mailbox))
            || !available(&mailbox)
            || self.commands.is_closed()
            || mailbox.guild != expected.panel
            || !mailbox.guild.open
        {
            return false;
        }
        let GuildWindowAction::Request(intent) = action else {
            mailbox.guild.open = false;
            return true;
        };
        if !permitted(&mailbox.guild, &intent) || needs_confirmation(&intent) && !confirmed {
            return false;
        }
        let mut candidate = mailbox.guild.clone();
        let Some(command) = request(&mut candidate, intent) else {
            return false;
        };
        if self.commands.send(PlayCommand::Guild(command)).is_err() {
            return false;
        }
        mailbox.guild = candidate;
        true
    }
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_guild_panel(&self, panel: GuildPanel) {
        lock(&self.0).guild = panel;
    }
    pub(crate) fn bevy_guild_reply(&self, reply: &oteryn_client_ui::guild::GuildReply) {
        oteryn_client_ui::guild::apply_result(&mut lock(&self.0).guild, reply);
    }
    pub(crate) fn bevy_guild_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(test)]
#[path = "bound_tests.rs"]
mod tests;
