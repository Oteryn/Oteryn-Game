use crate::play::{
    Outbox, Stepper,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::stash::StashResponse;

impl Outbox {
    pub(in crate::play) fn stash_publish(&self, session: &impl Stepper) {
        lock(&self.0).stash.carried = session.depot_items().inventory.entries;
    }
    pub(in crate::play) fn stash_initialize(&self, selected: bool) {
        lock(&self.0).stash.selected = selected;
    }
    pub(in crate::play) fn stash_result(&self, response: &StashResponse) {
        oteryn_client_ui::stash::apply_result(&mut lock(&self.0).stash, response);
    }
}

impl Mailbox {
    pub(in crate::play) fn stash_end(&mut self) {
        self.stash = oteryn_client_ui::stash::StashPanel {
            sort: self.stash.sort,
            sort_edit: self.stash.sort_edit,
            ..Default::default()
        };
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn bevy_stash_initialize(&self, selected: bool) {
        self.stash_initialize(selected);
    }
    pub(crate) fn bevy_stash_result(&self, response: &StashResponse) {
        self.stash_result(response);
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl Outbox {
    pub(crate) fn bevy_stash_carried(&self, items: Vec<oteryn_client_runtime::depot::ItemEntry>) {
        lock(&self.0).stash.carried = items;
    }
}
