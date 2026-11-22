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
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.cyclopedia_tracker;
        if !oteryn_client_ui::cyclopedia_tracker::apply_snapshot(panel, &payload) {
            panel.widget_feedback = [panel.feedback; 2];
            return;
        }
        let latest = panel.bosstiary_snapshot.clone();
        if let Some((old, new)) = mailbox.capture_boss_tracker.as_ref().zip(latest.as_ref())
            && new.entries.iter().any(|entry| {
                old.entries.iter().any(|before| {
                    before.creature_key == entry.creature_key
                        && before.kill_count < entry.kill_count
                })
            })
        {
            mailbox.combat_capture.boss_defeats =
                mailbox.combat_capture.boss_defeats.saturating_add(1);
        }
        mailbox.capture_boss_tracker = latest;
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
