use crate::play::{
    Outbox, Stepper,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::stash::StashResponse;

impl Outbox {
    pub(in crate::play) fn stash_publish(&self, session: &impl Stepper) {
        let sources = oteryn_client_ui::depot::DepotPanel {
            items: session.depot_items(),
            ..Default::default()
        };
        lock(&self.0).stash.carried = oteryn_client_ui::depot::deposit_items(&sources);
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
