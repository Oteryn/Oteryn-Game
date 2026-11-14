//! Atomic presentation bindings for the three existing read-only status owners.
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::{mechanics_status, party_effects, potion_durations};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatusDetailsTab {
    Party,
    Potions,
    Mechanics,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatusDetailsAction {
    Open(StatusDetailsTab),
    Refresh(StatusDetailsTab),
    Close(StatusDetailsTab),
}
#[derive(Clone, Debug)]
pub(crate) struct StatusDetailsPanel {
    session: Weak<Mutex<Mailbox>>,
    ended: bool,
    party: party_effects::PartyEffectsPanel,
    potions: potion_durations::PotionDurationsPanel,
    mechanics: mechanics_status::MechanicsStatusPanel,
}
impl PartialEq for StatusDetailsPanel {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
            && self.ended == other.ended
            && self.party == other.party
            && self.potions == other.potions
            && self.mechanics == other.mechanics
    }
}
impl Eq for StatusDetailsPanel {}
impl StatusDetailsPanel {
    fn read(link: &PlayLink, mailbox: &Mailbox) -> Self {
        Self {
            session: Arc::downgrade(&link.mailbox),
            ended: mailbox.quest_shutdown,
            party: mailbox.party_effects.clone(),
            potions: mailbox.potion_durations.clone(),
            mechanics: mailbox.mechanics_status.clone(),
        }
    }
    pub(crate) fn active(&self) -> Option<StatusDetailsTab> {
        if self.ended {
            return None;
        }
        if self.mechanics.open {
            Some(StatusDetailsTab::Mechanics)
        } else if self.potions.open {
            Some(StatusDetailsTab::Potions)
        } else {
            self.party.open.then_some(StatusDetailsTab::Party)
        }
    }
    pub(crate) fn ready(&self, tab: StatusDetailsTab) -> bool {
        !self.ended
            && match tab {
                StatusDetailsTab::Party => self.party.selected && !self.party.pending,
                StatusDetailsTab::Potions => self.potions.selected && !self.potions.pending,
                StatusDetailsTab::Mechanics => self.mechanics.selected && !self.mechanics.pending,
            }
    }
    pub(crate) fn text_at(&self, tab: StatusDetailsTab, now: std::time::Instant) -> String {
        match tab {
            StatusDetailsTab::Party => party_effects::panel_text(&self.party),
            StatusDetailsTab::Potions => potion_durations::panel_text_at(&self.potions, now),
            StatusDetailsTab::Mechanics => mechanics_status::panel_text(&self.mechanics),
        }
    }
}
impl PlayLink {
    pub(crate) fn status_details_panel(&self) -> StatusDetailsPanel {
        StatusDetailsPanel::read(self, &lock(&self.mailbox))
    }
    /// Revalidate the exact session and displayed owner state while sending the query.
    /// Closing and switching tabs preserve pending replies and last confirmed data.
    pub(crate) fn status_details_dispatch_bound(
        &self,
        expected: &StatusDetailsPanel,
        action: StatusDetailsAction,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let current = StatusDetailsPanel::read(self, &mailbox);
        if current != *expected
            || current.ended
            || self.commands.is_closed()
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.activity_reset_commands
        {
            return false;
        }
        let tab = match action {
            StatusDetailsAction::Open(tab) => {
                if current.active() == Some(tab) {
                    return false;
                }
                mailbox.party_effects.open = tab == StatusDetailsTab::Party;
                mailbox.potion_durations.open = tab == StatusDetailsTab::Potions;
                mailbox.mechanics_status.open = tab == StatusDetailsTab::Mechanics;
                tab
            }
            StatusDetailsAction::Close(tab) => {
                if current.active() != Some(tab) {
                    return false;
                }
                mailbox.party_effects.open = false;
                mailbox.potion_durations.open = false;
                mailbox.mechanics_status.open = false;
                return true;
            }
            StatusDetailsAction::Refresh(tab) => {
                if current.active() != Some(tab) || !current.ready(tab) {
                    return false;
                }
                tab
            }
        };
        // These are the canonical owner encoders and the existing Feature dispatcher.
        let (capability, kind, payload) = match tab {
            StatusDetailsTab::Party => {
                (280, 496, party_effects::request(&mut mailbox.party_effects))
            }
            StatusDetailsTab::Potions => (
                381,
                662,
                potion_durations::request(&mut mailbox.potion_durations),
            ),
            StatusDetailsTab::Mechanics => (
                384,
                669,
                mechanics_status::request(&mut mailbox.mechanics_status),
            ),
        };
        let Some(payload) = payload else {
            return matches!(action, StatusDetailsAction::Open(_));
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::Feature(
                oteryn_client_runtime::raw_feature::FeatureCommand {
                    capability,
                    kind,
                    payload,
                },
            ))
            .is_err()
        {
            mailbox.party_effects = current.party;
            mailbox.potion_durations = current.potions;
            mailbox.mechanics_status = current.mechanics;
            return false;
        }
        true
    }
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn status_details_test_selected(&self) {
        self.party_effects_initialize(true);
        self.potion_durations_initialize(true);
        self.mechanics_status_initialize(true);
    }
    pub(crate) fn status_details_test_reply(
        &self,
        tab: StatusDetailsTab,
        accepted: bool,
        payload: &[u8],
    ) {
        let result = oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: if accepted {
                oteryn_session::CommandStatus::Accepted
            } else {
                oteryn_session::CommandStatus::Rejected
            },
            payload: payload.to_vec(),
        };
        match tab {
            StatusDetailsTab::Party => self.party_effects_result(&result),
            StatusDetailsTab::Potions => self.potion_durations_result(&result),
            StatusDetailsTab::Mechanics => self.mechanics_status_result(&result),
        }
    }
    pub(crate) fn status_details_test_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}

#[cfg(test)]
#[path = "status_details_tests.rs"]
mod tests;
