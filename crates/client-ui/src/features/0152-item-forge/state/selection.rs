//! Bounded observations and pair-selection state; server quotes never authorize a write.
use super::ItemForgePanel;
use oteryn_client_runtime::cui3::item_forge::{Operation, Request};
use oteryn_client_runtime::{depot::ItemEntry, forge_preview as wire};
pub const MAX_CANDIDATES: usize = 501;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviewState {
    pub selected: bool,
    pub nonce: u64,
    pub pending: Option<wire::Request>,
    pub page: Option<wire::Page>,
    pub items: Vec<wire::Item>,
    pub live: Vec<ItemEntry>,
    pub quote: Option<wire::Quote>,
    pub refresh: bool,
    pub navigation: Option<u64>,
    pub feedback: Option<String>,
}
impl PreviewState {
    pub fn item(&self, handle: u64) -> Option<&wire::Item> {
        self.items.iter().find(|item| item.handle.get() == handle)
    }
    pub fn remember(&mut self, item: wire::Item) -> bool {
        if let Some(old) = self.items.iter_mut().find(|old| old.handle == item.handle) {
            *old = item;
            true
        } else if self.items.len() < MAX_CANDIDATES {
            self.items.push(item);
            true
        } else {
            false
        }
    }
    pub fn invalidate(&mut self) {
        self.nonce = self.nonce.saturating_add(1);
        self.pending = None;
        self.quote = None;
    }
}
impl ItemForgePanel {
    pub fn selection(&self) -> Option<wire::Selection> {
        Some(wire::Selection::Pair {
            operation: self.operation,
            first: oteryn_client_runtime::depot::ItemHandle::new(self.first_item)?,
            second: oteryn_client_runtime::depot::ItemHandle::new(self.second_item)?,
            boost: self.boost,
            protect: self.protect,
        })
    }
    pub fn editing(&self) -> bool {
        self.selected && !self.pending && self.retry.is_none() && !self.resource_dialog.confirming()
    }
    pub fn choose_operation(&mut self, operation: Operation) -> bool {
        if !self.editing() || !operation.requires_pair() {
            return false;
        }
        self.operation = operation;
        self.query_current_dust = false;
        if operation != Operation::Fusion {
            self.boost = false;
            self.protect = false;
        }
        self.preview.invalidate();
        true
    }
    pub fn choose_item(&mut self, first: bool, handle: u64) -> bool {
        if !self.editing()
            || if self.preview.selected {
                self.preview.item(handle).is_none_or(|item| item.container)
            } else {
                !self
                    .preview
                    .live
                    .iter()
                    .any(|item| item.handle.get() == handle)
            }
        {
            return false;
        }
        if first {
            self.first_item = handle;
        } else {
            self.second_item = handle;
        }
        self.preview.invalidate();
        true
    }
    pub fn choose_cores(&mut self, boost: bool, protect: bool) -> bool {
        if !self.editing() || self.operation != Operation::Fusion {
            return false;
        }
        self.boost = boost;
        self.protect = protect;
        self.preview.invalidate();
        true
    }
    pub fn verified_quote(&self) -> bool {
        self.selection().is_some_and(|selection| {
            self.preview
                .quote
                .as_ref()
                .is_some_and(|quote| quote.selection == selection)
        })
    }
    pub fn paid_request(&mut self, occurrence: [u8; 16]) -> Option<Request> {
        if !self.editing() || self.preview.pending.is_some() || !self.operation.requires_pair() {
            return None;
        }
        if self.preview.selected && !self.verified_quote() {
            self.feedback = Some("Request and review a current server quote first".into());
            return None;
        }
        let occurrence_valid = occurrence[6] >> 4 == 7 && occurrence[8] & 192 == 128;
        if !occurrence_valid {
            return None;
        }
        self.selection()?.canonical(occurrence)
    }
    pub fn scope_changed(&mut self) {
        self.resource_dialog.reset();
        self.pending = false;
        self.submitted = None;
        self.preview.invalidate();
        self.preview.items.clear();
        self.preview.live.clear();
        self.preview.page = None;
        self.preview.navigation = None;
        self.preview.refresh = false;
        self.first_item = 0;
        self.second_item = 0;
        self.feedback=Some(if self.retry.is_some(){"Scope changed. Retry only the same unresolved attempt; refresh before a new action."}else{"Scope changed; refresh carried items and quotes"}.into());
    }
    pub fn observe(&mut self, live: Vec<ItemEntry>) {
        if self.preview.live == live {
            return;
        }
        self.preview.items.retain(|item| {
            live.iter().any(|entry| {
                entry.handle == item.handle
                    && entry.item_definition_ref.get() == item.definition_ref
            })
        });
        if !live
            .iter()
            .any(|entry| entry.handle.get() == self.first_item)
        {
            self.first_item = 0;
        }
        if !live
            .iter()
            .any(|entry| entry.handle.get() == self.second_item)
        {
            self.second_item = 0;
        }
        if self.resource_dialog.confirming() {
            self.resource_dialog.reset();
        }
        self.preview.live = live;
        self.preview.invalidate();
        self.preview.page = None;
        self.preview.refresh = true;
    }
}
