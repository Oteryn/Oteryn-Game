//! Atomic shared-frontend operations. Rejected actions leave both owners untouched.
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::{
    purchase_history,
    store::{self, StorePage, StorePanel},
};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone)]
pub(crate) struct StoreSnapshot {
    session: Weak<Mutex<Mailbox>>,
    pub panel: StorePanel,
    pub history: purchase_history::Panel,
}
impl StoreSnapshot {
    pub(crate) fn same_session(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
    }
}
impl PartialEq for StoreSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
            && self.panel == other.panel
            && self.history == other.history
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoreCustomization {
    pub name: String,
    pub sex: u8,
    pub identity: Option<[u8; 16]>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StoreAction {
    Close,
    GiftOpen,
    Browse {
        category: String,
        search: String,
        sort: u8,
        step: i8,
    },
    Select(usize),
    Purchase(StoreCustomization),
    History(bool),
    Ledger(bool),
    Filter {
        ledger: bool,
        key: String,
        from: String,
        until: String,
    },
    Gift {
        action: usize,
        fields: [String; 2],
    },
}
impl PlayLink {
    pub(crate) fn store_snapshot(&self) -> StoreSnapshot {
        let m = lock(&self.mailbox);
        StoreSnapshot {
            session: Arc::downgrade(&self.mailbox),
            panel: m.store.clone(),
            history: m.discovery.history.clone(),
        }
    }
    pub(crate) fn store_open_bound(&self) -> bool {
        let mut m = lock(&self.mailbox);
        if self.commands.is_closed()
            || m.depot_shutdown
            || m.server_modal.is_active()
            || m.world_input_blocked()
            || !m.store.selected
        {
            return false;
        }
        m.store.open = true;
        true
    }
    /// The expected snapshot is the consent's exact quote and originating session.
    pub(crate) fn store_dispatch_bound(
        &self,
        expected: &StoreSnapshot,
        action: StoreAction,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if !expected.session.ptr_eq(&Arc::downgrade(&self.mailbox))
            || self.commands.is_closed()
            || m.depot_shutdown
            || m.server_modal.is_active()
            || m.world_input_blocked()
            || m.store != expected.panel
            || m.discovery.history != expected.history
            || !m.store.open
            || !m.store.selected
            || m.store.pending
            || m.discovery.history.pending
            || m.store.gift.pending.is_some()
        {
            return false;
        }
        let mut panel = m.store.clone();
        let mut history = m.discovery.history.clone();
        let command = match action {
            StoreAction::GiftOpen => {
                panel.gift_open = true;
                None
            }
            StoreAction::Close => {
                panel.open = false;
                None
            }
            StoreAction::Browse {
                category,
                search,
                sort,
                step,
            } => {
                // Initial browse uses the owner's default; all later categories must be server-provided.
                if panel
                    .browser
                    .page
                    .as_ref()
                    .map_or(category != panel.browser.category, |page| {
                        !page.categories.iter().any(|c| c.key == category)
                    })
                {
                    return false;
                }
                panel.gift_open = false;
                panel.page = StorePage::Catalogue;
                let Some(c) = store::browse(&mut panel, &category, &search, sort, step) else {
                    return false;
                };
                Some(c)
            }
            StoreAction::Select(index) => {
                if !store::select_product(&mut panel, index) {
                    return false;
                }
                None
            }
            StoreAction::Purchase(custom) => {
                // Uncertain purchases must retain their original bytes; a new review cannot replace them.
                if panel.confirmed_purchase.is_some() || panel.gift.confirmed.is_some() {
                    return false;
                }
                let Some(offer) = panel
                    .catalogue
                    .as_ref()
                    .and_then(|p| p.offers.get(panel.selected_offer))
                else {
                    return false;
                };
                if offer.currency == "COSMETIC_TOKEN" && !panel.token_supported {
                    return false;
                }
                // Name/sex changes use an explicit reviewed identity rather than a later selection modal.
                if matches!(
                    offer.offer_key.as_str(),
                    "store:hireling:25438" | "store:hireling:25437"
                ) && custom.identity.is_none()
                {
                    return false;
                }
                let Some(id) = super::super::input::gift::operation() else {
                    return false;
                };
                panel.purchase_request_id = id;
                panel.hireling_name = custom.name;
                panel.hireling_sex = custom.sex;
                panel.hireling_id = custom.identity;
                panel.purchase_confirmed = true;
                let Some(c) = store::purchase(&mut panel) else {
                    return false;
                };
                Some(c)
            }
            StoreAction::History(next) => {
                let Some(c) = purchase_history::protocol::request(&mut history, next) else {
                    return false;
                };
                panel.gift_open = false;
                panel.page = StorePage::History;
                Some(c)
            }
            StoreAction::Ledger(next) => {
                panel.gift_open = false;
                let Some(c) = store::coin_history(&mut panel, next) else {
                    return false;
                };
                Some(c)
            }
            StoreAction::Filter {
                ledger,
                key,
                from,
                until,
            } => {
                let accepted = if ledger {
                    store::coin_history_filter(&mut panel, &key, &from, &until)
                } else {
                    purchase_history::set_filter(&mut history, &key, &from, &until)
                };
                if !accepted {
                    return false;
                }
                None
            }
            StoreAction::Gift { action, fields } => {
                if !matches!(action, 0 | 1 | 3 | 4) || panel.confirmed_purchase.is_some() {
                    return false;
                }
                let operation = if action == 1 {
                    let Some(id) = super::super::input::gift::operation() else {
                        return false;
                    };
                    id
                } else {
                    [0; 16]
                };
                let Some(c) = store::gift_control(&mut panel, action, fields, operation) else {
                    return false;
                };
                Some(c)
            }
        };
        if let Some(command) = command
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            return false;
        }
        m.store = panel;
        m.discovery.history = history;
        true
    }
}
