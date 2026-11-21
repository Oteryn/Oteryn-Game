use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn cyclopedia_tracker_initialize(&self, selected: bool) {
        lock(&self.0).cyclopedia_tracker.selected = selected;
    }
    pub(in crate::play) fn cyclopedia_tracker_result(&self, result: &GatedCommandResult) {
        let panel = &mut lock(&self.0).cyclopedia_tracker;
        // The answered request is the one sent last, for the tracker `boss` names.
        let slot = usize::from(panel.boss);
        oteryn_client_ui::cyclopedia_tracker::apply_result(panel, result);
        panel.widget_feedback[slot] = panel.feedback;
    }
    pub(in crate::play) fn cyclopedia_tracker_push(&self, payload: Vec<u8>) {
        let panel = &mut lock(&self.0).cyclopedia_tracker;
        if !oteryn_client_ui::cyclopedia_tracker::apply_snapshot(panel, &payload) {
            // A domain-98 push carries both trackers.
            panel.widget_feedback = [panel.feedback; 2];
        }
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
