#[path = "bound/retry.rs"]
mod retry;
pub(crate) use retry::HirelingRetry;
// Atomic presentation bindings over the existing hireling and Store owners.
use super::super::input::configuration;
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::{
    hireling_management::{self as ui, Intent, Panel},
    store::StorePanel,
};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone, Debug)]
pub(crate) struct HirelingSnapshot {
    session: Weak<Mutex<Mailbox>>,
    pub panel: Panel,
    pub store: StorePanel,
    pub live: bool,
}
impl HirelingSnapshot {
    pub(crate) fn same_session(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
    }
}
impl PartialEq for HirelingSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.same_session(other)
            && self.panel == other.panel
            && self.store == other.store
            && self.live == other.live
    }
}
impl Eq for HirelingSnapshot {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HirelingReview(configuration::Review);
impl HirelingReview {
    pub fn text(&self) -> String {
        self.0.text()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HirelingAction {
    Open,
    Close,
    Select(usize),
    Refresh,
    Return,
    Outfit { look_type: u16, colors: [u8; 4] },
    Prices,
    Configure(Box<HirelingReview>),
    Retry(Box<HirelingRetry>),
}
impl PlayLink {
    pub(crate) fn hireling_management_snapshot(&self) -> HirelingSnapshot {
        let m = lock(&self.mailbox);
        HirelingSnapshot {
            session: Arc::downgrade(&self.mailbox),
            panel: m.hireling_management.clone(),
            store: m.store.clone(),
            // The existing mailbox lifetime flag is set by end, logout and PlayLink::drop.
            live: !m.hireling_shutdown && !m.custom_hotkeys_ended && !self.commands.is_closed(),
        }
    }
    fn hireling_management_matches(&self, m: &Mailbox, expected: &HirelingSnapshot) -> bool {
        expected.live
            && expected.session.ptr_eq(&Arc::downgrade(&self.mailbox))
            && !m.hireling_shutdown
            && !m.custom_hotkeys_ended
            && !m.activity_reset_commands
            && !self.commands.is_closed()
            && m.hireling_management == expected.panel
            && m.store == expected.store
            && !m.server_modal.is_active()
            && !m.world_input_blocked()
    }
    pub(crate) fn hireling_management_review_bound(
        &self,
        expected: &HirelingSnapshot,
        action: usize,
        name: &str,
    ) -> Result<HirelingReview, &'static str> {
        let m = lock(&self.mailbox);
        if !m.hireling_management.open || !self.hireling_management_matches(&m, expected) {
            return Err("Hireling configuration unavailable in this session");
        }
        configuration::review(&m, action, name).map(HirelingReview)
    }
    pub(crate) fn hireling_management_dispatch_bound(
        &self,
        expected: &HirelingSnapshot,
        action: HirelingAction,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if !self.hireling_management_matches(&m, expected) {
            return false;
        }
        if action == HirelingAction::Open {
            if m.hireling_management.open {
                return false;
            }
            m.hireling_management.open = true;
            if m.hireling_management.pending.is_some() || m.store.pending {
                return true;
            }
            if let Some(command) = ui::request(&mut m.hireling_management, Intent::List)
                && self.commands.send(PlayCommand::Feature(command)).is_err()
            {
                m.hireling_management = expected.panel.clone();
                return false;
            }
            return true;
        }
        if !m.hireling_management.open {
            return false;
        }
        if action == HirelingAction::Close {
            m.hireling_management.open = false;
            return true;
        }
        if let HirelingAction::Retry(review) = action {
            return retry::submit(&mut m, &self.commands, &review);
        }
        if !m.hireling_management.selected
            || m.hireling_management.pending.is_some()
            || m.store.pending
        {
            return false;
        }
        match action {
            HirelingAction::Open | HirelingAction::Close | HirelingAction::Retry(_) => false, // Handled before the pending gate so an in-flight window can close.
            HirelingAction::Select(index) => ui::select(&mut m.hireling_management, index),
            HirelingAction::Configure(review) => {
                if configuration::review(&m, review.0.action, &review.0.name).as_ref()
                    != Ok(&review.0)
                {
                    return false;
                }
                if configuration::submit(&mut m, &self.commands, &review.0, true) {
                    true
                } else {
                    m.hireling_management = expected.panel.clone();
                    m.store = expected.store.clone();
                    false
                }
            }
            HirelingAction::Prices => {
                if !m.store.selected || m.store.confirmed_purchase.is_some() {
                    return false;
                }
                configuration::prices(&mut m, &self.commands);
                if m.store.pending {
                    true
                } else {
                    m.hireling_management = expected.panel.clone();
                    m.store = expected.store.clone();
                    false
                }
            }
            action => {
                let intent = match action {
                    HirelingAction::Refresh => Intent::List,
                    HirelingAction::Return => {
                        if !m
                            .hireling_management
                            .entries
                            .iter()
                            .any(|e| Some(e.hireling) == m.hireling_management.identity)
                        {
                            return false;
                        }
                        Intent::Return
                    }
                    HirelingAction::Outfit { look_type, colors } => {
                        if !m
                            .hireling_management
                            .entries
                            .iter()
                            .find(|e| Some(e.hireling) == m.hireling_management.identity)
                            .is_some_and(|e| {
                                ui::outfits(e).iter().any(|(_, look)| *look == look_type)
                            })
                        {
                            return false;
                        }
                        Intent::Outfit { look_type, colors }
                    }
                    _ => return false,
                };
                let Some(command) = ui::request(&mut m.hireling_management, intent) else {
                    return false;
                };
                if self.commands.send(PlayCommand::Feature(command)).is_ok() {
                    true
                } else {
                    m.hireling_management = expected.panel.clone();
                    false
                }
            }
        }
    }
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_hireling_management_initialize(&self) {
        self.hireling_management_initialize(true);
        self.store_initialize(true, true);
    }
    pub(crate) fn bevy_hireling_management_store_reply(&self, payload: Vec<u8>) {
        self.bevy_hireling_management_store_status(
            oteryn_session::CommandStatus::Accepted,
            payload,
        );
    }
    pub(crate) fn bevy_hireling_management_store_status(
        &self,
        status: oteryn_session::CommandStatus,
        payload: Vec<u8>,
    ) {
        self.store_result(
            &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                command_id: 1,
                server_sequence: 1,
                status,
                payload,
            },
        );
    }
    pub(crate) fn bevy_hireling_management_reply(&self, reply: ui::wire::Reply) {
        if let Some(payload) = reply.encode() {
            self.hireling_management_result(
                &oteryn_client_runtime::raw_feature::GatedCommandResult {
                    command_id: 1,
                    server_sequence: 1,
                    status: oteryn_session::CommandStatus::Accepted,
                    payload,
                },
            );
        }
    }
}
#[cfg(test)]
#[path = "bound/tests.rs"]
mod tests;
