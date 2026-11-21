use crate::play::{
    Outbox, Pushed, Stepper,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::achievement_notices::{
    AchievementEarned, decode_achievement_notices_snapshot, reduce,
};
use oteryn_session::AppliedDelta;

impl Outbox {
    /// Initialize from the validated join snapshot before publishing typed live events.
    pub(in crate::play) fn achievement_notices_publish<T: Stepper>(&self, session: &mut T) {
        {
            let mut mailbox = lock(&self.0);
            let panel = &mut mailbox.achievement_notices;
            if panel.watermark.is_none() {
                panel.selected = session.feature_selected(8);
                panel.watermark = session
                    .feature_snapshot(13)
                    .and_then(|payload| decode_achievement_notices_snapshot(&payload).ok());
            }
        }
        let mut preceding = Vec::new();
        for event in session.drain() {
            if let oteryn_session::SessionEvent::GatedSnapshotReplaced(delta) = &event
                && delta.value.domain_id == 13
                && delta.value.snapshot_type == 1
                && let Ok(watermark) = decode_achievement_notices_snapshot(&delta.value.payload)
            {
                // Preserve event order: earlier notices cannot overwrite the recovery totals.
                self.push(std::mem::take(&mut preceding));
                lock(&self.0).achievement_notices.watermark = Some(watermark);
            }
            preceding.push(event);
        }
        self.push(preceding);
    }
}

pub(in crate::play) fn achievement_earned(
    pushed: &mut Pushed,
    delta: AppliedDelta<AchievementEarned>,
) {
    pushed.achievements.push(delta.value);
}

impl Mailbox {
    pub(in crate::play) fn reduce_achievement_notices(&mut self, notices: Vec<AchievementEarned>) {
        for notice in notices {
            #[cfg(bevy_shell)]
            if self.achievement_notices.selected {
                self.achievement_banners.push_back(
                    oteryn_client_ui::achievement_notices::advancement_text(&notice.name),
                );
            }
            reduce(&mut self.achievement_notices, notice);
        }
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn achievement_notices_select_for_test(&self) {
        lock(&self.0).achievement_notices.selected = true;
    }
}
