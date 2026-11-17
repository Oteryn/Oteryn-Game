use super::AchievementNoticesPanel;

pub fn notices_panel_text(panel: &AchievementNoticesPanel) -> String {
    if !panel.selected {
        return tr!("Achievement notices unavailable in this session").into();
    }
    let Some(totals) = panel.watermark else {
        return tr!("Waiting for achievement totals…").into();
    };
    let mut text = lfmt!(
        "Achievements: {} · Total points: {}\r\n\r\nRecent notices (this session, latest 32):\r\n",
        totals.fact_count,
        totals.total_points
    );
    if panel.recent.is_empty() {
        text.push_str(tr!("No achievements earned this session.\r\n"));
    }
    for notice in &panel.recent {
        text.push_str(&lfmt!("Achievement earned: {}\r\n", notice.name));
    }
    text
}

use super::AchievementNotices;

#[must_use]
pub fn panel_text(notices: &AchievementNotices) -> String {
    if !notices.selected {
        return tr!("Achievement notices unavailable in this session").into();
    }
    if notices.advancement_log().len() == 0 {
        return tr!("No achievements earned this session").into();
    }
    notices.advancement_log().collect::<Vec<_>>().join("\r\n")
}

#[must_use]
pub fn popup_text(notices: &AchievementNotices) -> Option<String> {
    notices.popup().map(|popup| popup.text.clone())
}
