use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::stash::StashResponse;

impl Outbox {
    pub(in crate::play) fn stash_initialize(&self, selected: bool) {
        lock(&self.0).stash.selected = selected;
    }
    pub(in crate::play) fn stash_result(&self, response: &StashResponse) {
        oteryn_client_ui::stash::apply_result(&mut lock(&self.0).stash, response);
    }
}
