use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::party::PartyPanel;

impl PlayLink {
    #[must_use]
    pub fn party_panel(&self) -> PartyPanel {
        lock(&self.mailbox).party.clone()
    }
}

#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::context_menu;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(feature = "bevy-shell")]
#[derive(Clone, Copy)]
pub(crate) enum PartyViewAction {
    Member(u64),
    Invitation(u64),
    Request(oteryn_client_ui::party::Intent),
}
impl PlayLink {
    #[must_use]
    pub fn party_is_open(&self) -> bool {
        lock(&self.mailbox).party.open
    }
    pub fn party_close(&self) {
        lock(&self.mailbox).party.open = false;
    }
    /// Re-proves the displayed typed view before selecting a handle or preparing a request.
    #[cfg(feature = "bevy-shell")]
    pub(crate) fn party_action_bound(
        &self,
        expected: &PartyPanel,
        action: PartyViewAction,
    ) -> bool {
        use oteryn_client_ui::party::Intent;
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.party;
        if !panel.selected
            || !expected.selected
            || panel.pending.is_some()
            || panel.state != expected.state
            || panel.own_character_name != expected.own_character_name
            || panel.chat_selected != expected.chat_selected
        {
            return false;
        }
        if !panel.open && !matches!(action, PartyViewAction::Request(Intent::Invite(_))) {
            return false;
        }
        match action {
            PartyViewAction::Member(handle) => {
                if !panel
                    .state
                    .as_ref()
                    .is_some_and(|state| state.members.iter().any(|entry| entry.handle == handle))
                {
                    return false;
                }
                panel.selected_member = Some(handle);
                return true;
            }
            PartyViewAction::Invitation(handle) => {
                if !panel.state.as_ref().is_some_and(|state| {
                    state.invitations.iter().any(|entry| entry.handle == handle)
                }) {
                    return false;
                }
                panel.selected_invitation = Some(handle);
                return true;
            }
            PartyViewAction::Request(intent) => {
                if matches!(intent, Intent::Kick | Intent::PassLeadership)
                    && panel.selected_member != expected.selected_member
                    || matches!(intent, Intent::Join)
                        && panel.selected_invitation != expected.selected_invitation
                {
                    return false;
                }
                let Some(command) = oteryn_client_ui::party::request(panel, intent) else {
                    return false;
                };
                if self
                    .commands
                    .send(crate::combat_input::PlayCommand::Party(command))
                    .is_err()
                {
                    panel.pending = None;
                    panel.feedback = Some(tr!("The session has ended"));
                    return false;
                }
                if matches!(intent, Intent::Invite(_)) {
                    panel.open = true;
                }
            }
        }
        true
    }
}
#[cfg(feature = "bevy-shell")]
impl crate::play::PlayView {
    /// A context label is only the exact visible player's server-supplied display name.
    pub(crate) fn party_context_target(
        &self,
        tile: oteryn_renderer::TileCoord,
    ) -> Option<(oteryn_session::EntityRef, Option<String>)> {
        let entity = self.party_target(tile)?;
        let name = self
            .entities
            .iter()
            .find(|entry| entry.entity == entity)
            .and_then(|entry| entry.display_name.as_ref())
            .and_then(|name| name.as_str().ok())
            .map(str::to_owned);
        Some((entity, name))
    }
}
