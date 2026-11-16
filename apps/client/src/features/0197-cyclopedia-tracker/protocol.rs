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
        oteryn_client_ui::cyclopedia_tracker::apply_snapshot(
            &mut mailbox.cyclopedia_tracker,
            &payload,
        );
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    /// Admits capability 197 for the Bevy tracker window tests.
    pub(crate) fn bevy_cyclopedia_tracker_initialize(&self) {
        self.cyclopedia_tracker_initialize(true);
    }
    /// Applies a tracker command reply through the session's reducer.
    pub(crate) fn bevy_cyclopedia_tracker_result(&self, result: &GatedCommandResult) {
        self.cyclopedia_tracker_result(result);
    }
    /// Applies a pushed domain-98 tracker snapshot through the session's reducer.
    pub(crate) fn bevy_cyclopedia_tracker_push(&self, payload: Vec<u8>) {
        self.cyclopedia_tracker_push(payload);
    }
}
