use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::party::PartyResponse;

impl Outbox {
    pub(in crate::play) fn party_initialize(&self, selected: bool) {
        lock(&self.0).party.selected = selected;
    }

    pub(in crate::play) fn party_result(&self, response: &PartyResponse) {
        oteryn_client_ui::party::apply_result(&mut lock(&self.0).party, response);
    }
}
