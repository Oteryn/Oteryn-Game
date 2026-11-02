use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn cyclopedia_tracker_initialize(&self, selected: bool) {
        lock(&self.0).cyclopedia_tracker.selected = selected;
    }
    pub(in crate::play) fn cyclopedia_tracker_result(&self, result: &GatedCommandResult) {
        oteryn_client_ui::cyclopedia_tracker::apply_result(
            &mut lock(&self.0).cyclopedia_tracker,
            result,
        );
    }
    pub(in crate::play) fn cyclopedia_tracker_push(&self, payload: Vec<u8>) {
        let mut mailbox = lock(&self.0);
        if mailbox.cyclopedia_tracker.result != payload {
            mailbox.cyclopedia_tracker.result = payload;
        }
    }
}
