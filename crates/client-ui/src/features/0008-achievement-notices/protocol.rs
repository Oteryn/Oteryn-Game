use super::{AchievementEarned, AchievementNoticesPanel};

pub fn reduce(panel: &mut AchievementNoticesPanel, notice: AchievementEarned) {
    if !panel.selected {
        return;
    }
    panel.watermark = Some(notice.watermark);
    if panel.recent.len() == 32 {
        panel.recent.pop_front();
    }
    panel.recent.push_back(notice);
}

use super::AchievementNotices;

pub fn apply(
    notices: &mut AchievementNotices,
    earned: oteryn_client_runtime::achievement_notices::AchievementEarned,
) {
    notices.push(earned.name);
}
