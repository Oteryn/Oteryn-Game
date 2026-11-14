//! Atomic Bevy presentation entry point over the existing discovery owners.
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
};
use oteryn_client_ui::{team_finder as teams, team_finder_leader as leader};
use std::sync::{Arc, Mutex, Weak};

#[derive(Debug, Clone)]
pub struct TeamFinderSnapshot {
    session: Weak<Mutex<crate::play::mailbox::Mailbox>>,
    pub teams: teams::Panel,
    pub leader: leader::Panel,
}
impl PartialEq for TeamFinderSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
            && self.teams == other.teams
            && self.leader == other.leader
    }
}
impl Eq for TeamFinderSnapshot {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamFinderAction {
    Close,
    Select(usize),
    Filter(teams::TeamFilter),
    Browse { applicants: bool, next: bool },
    Member { cancel: bool },
    Decide(u8),
    Advertise(Option<leader::LeaderListing>),
}
impl TeamFinderAction {
    pub fn needs_confirmation(&self) -> bool {
        matches!(
            self,
            Self::Member { cancel: true } | Self::Decide(4) | Self::Advertise(None)
        )
    }
}
fn snapshot(shared: &SharedMailbox, mailbox: &crate::play::mailbox::Mailbox) -> TeamFinderSnapshot {
    TeamFinderSnapshot {
        session: Arc::downgrade(shared),
        teams: mailbox.discovery.teams.clone(),
        leader: mailbox.discovery.leader.clone(),
    }
}
impl PlayLink {
    pub fn team_finder_snapshot(&self) -> TeamFinderSnapshot {
        snapshot(&self.mailbox, &lock(&self.mailbox))
    }
    pub fn team_finder_dispatch_bound(
        &self,
        expected: &TeamFinderSnapshot,
        action: TeamFinderAction,
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if snapshot(&self.mailbox, &mailbox) != *expected
            || !mailbox.discovery.teams.open
            || mailbox.custom_hotkeys_ended
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || (action.needs_confirmation() && !confirmed)
        {
            return false;
        }
        if action != TeamFinderAction::Close
            && (!mailbox.discovery.teams.selected
                || mailbox.discovery.teams.pending.is_some()
                || mailbox.discovery.leader.pending)
        {
            return false;
        }
        let state = &mut mailbox.discovery;
        let before_teams = state.teams.clone();
        let before_leader = state.leader.clone();
        let command = match action {
            TeamFinderAction::Close => {
                state.teams.open = false;
                return true;
            }
            TeamFinderAction::Select(row) => {
                if row >= state.teams.visible_names().len() {
                    return false;
                }
                state.teams.selected_row = row;
                return true;
            }
            TeamFinderAction::Filter(filter) => {
                if filter.target > 3 {
                    return false;
                }
                state.teams.filter = filter;
                state.teams.selected_row = 0;
                return true;
            }
            TeamFinderAction::Browse { applicants, next } => {
                teams::query(&mut state.teams, applicants, next)
            }
            TeamFinderAction::Member { cancel } => teams::member(&mut state.teams, cancel),
            TeamFinderAction::Decide(action) => teams::leader(&mut state.teams, action),
            TeamFinderAction::Advertise(listing) => {
                leader::protocol::request(&mut state.leader, listing)
            }
        };
        let Some(command) = command else {
            return false;
        };
        // The existing query owner clears old data while waiting. Keep the accepted page
        // visible until its reply; pending still fences all further actions.
        state.teams.result.clone_from(&before_teams.result);
        if self
            .commands
            .send(crate::combat_input::PlayCommand::ServerMechanic(command))
            .is_err()
        {
            state.teams = before_teams;
            state.leader = before_leader;
            return false;
        }
        true
    }
}

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_team_finder_state(&self, teams: teams::Panel, leader: leader::Panel) {
        let mut mailbox = lock(&self.0);
        mailbox.discovery.teams = teams;
        mailbox.discovery.leader = leader;
    }
    pub(crate) fn bevy_team_finder_response(
        &self,
        capability: u32,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.discovery_result(capability, response);
    }
}
