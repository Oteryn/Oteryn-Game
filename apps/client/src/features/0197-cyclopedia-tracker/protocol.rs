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
        let old_bosses = mailbox.capture_boss_tracker.take();
        let panel = &mut mailbox.cyclopedia_tracker;
        let old_bestiary = panel.bestiary_snapshot.clone();
        if !oteryn_client_ui::cyclopedia_tracker::apply_snapshot(panel, &payload) {
            panel.widget_feedback = [panel.feedback; 2];
            mailbox.capture_boss_tracker = old_bosses;
            return;
        }
        let mut events = [false; 3];
        if let Some((old, new)) = old_bestiary.as_ref().zip(panel.bestiary_snapshot.as_ref()) {
            for entry in &new.entries {
                if let Some(before) = old
                    .entries
                    .iter()
                    .find(|e| e.creature_key == entry.creature_key)
                    && before.thresholds == entry.thresholds
                    && entry.kill_count > before.kill_count
                {
                    if entry.completed && !before.completed {
                        events[1] = true;
                    } else if entry
                        .thresholds
                        .iter()
                        .any(|&n| before.kill_count < n && entry.kill_count >= n)
                    {
                        events[0] = true;
                    }
                }
            }
        }
        if let Some((old, new)) = old_bosses.as_ref().zip(panel.bosstiary_snapshot.as_ref()) {
            events[2] = new.entries.iter().any(|entry| {
                old.entries.iter().any(|before| {
                    before.creature_key == entry.creature_key
                        && entry.kill_count > before.kill_count
                })
            });
        }
        let latest = panel.bosstiary_snapshot.clone();
        for (counter, event) in mailbox.capture_observations.tracker.iter_mut().zip(events) {
            *counter = counter.wrapping_add(u64::from(event));
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
