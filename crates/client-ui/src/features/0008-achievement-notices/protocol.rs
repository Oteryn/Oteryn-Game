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
