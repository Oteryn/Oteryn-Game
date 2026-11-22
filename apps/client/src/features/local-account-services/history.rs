//! Verified observations belong to one authorization and outlive changes of selection.
use oteryn_platform_client::native_login::{
    bazaar::{OperationReceipt, OperationState},
    coin_purchase::CoinOrder,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(crate) trait HistoryReceipt: Clone + PartialEq {
    fn request_id(&self) -> &str;
    fn terminal(&self) -> bool;
    fn consistent_with(&self, old: &Self) -> bool;
}

impl HistoryReceipt for OperationReceipt {
    fn request_id(&self) -> &str {
        &self.request_id
    }
    fn terminal(&self) -> bool {
        self.state != OperationState::Pending
    }
    fn consistent_with(&self, old: &Self) -> bool {
        !old.terminal() || old == self
    }
}

impl HistoryReceipt for CoinOrder {
    fn request_id(&self) -> &str {
        &self.request_id
    }
    fn terminal(&self) -> bool {
        self.state.completed()
    }
    fn consistent_with(&self, old: &Self) -> bool {
        self.order_id == old.order_id
            && self.package_id == old.package_id
            && self.quote_id == old.quote_id
            && self.character_id == old.character_id
            && self.coins == old.coins
            && self.price == old.price
            && self.revision > old.revision
            && self.state.can_follow(old.state)
    }
}

#[derive(Clone)]
pub(crate) struct HistoryStatus<T> {
    pub(crate) unverified: BTreeSet<String>,
    verified: Arc<BTreeMap<String, T>>,
}

impl<T> Default for HistoryStatus<T> {
    fn default() -> Self {
        Self {
            unverified: Default::default(),
            verified: Default::default(),
        }
    }
}

impl<T: HistoryReceipt> HistoryStatus<T> {
    pub(crate) fn known_open(&self, id: &str) -> bool {
        self.verified
            .get(id)
            .is_some_and(|receipt| !receipt.terminal())
    }

    pub(crate) fn lookup_failed(&mut self, id: String) {
        if !self.verified.get(&id).is_some_and(HistoryReceipt::terminal) {
            self.unverified.insert(id);
        }
    }

    pub(crate) fn record(&mut self, receipt: &T) -> bool {
        let id = receipt.request_id();
        if let Some(old) = self.verified.get(id) {
            if old == receipt {
                return true;
            }
            if !receipt.consistent_with(old) {
                return false;
            }
        }
        if receipt.terminal() {
            self.unverified.remove(id);
        }
        Arc::make_mut(&mut self.verified).insert(id.to_owned(), receipt.clone());
        true
    }
}

impl<T> std::fmt::Debug for HistoryStatus<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryStatus")
            .field("unverified", &self.unverified.len())
            .finish_non_exhaustive()
    }
}
