use super::AchievementNoticesPanel;

pub fn panel_text(panel: &AchievementNoticesPanel) -> String {
    if !panel.selected {
        return "Achievement notices unavailable in this session".into();
    }
    let Some(totals) = panel.watermark else {
        return "Waiting for achievement totals…".into();
    };
    let mut text = format!(
        "Achievements: {} · Total points: {}\r\n\r\nRecent notices (this session, latest 32):\r\n",
        totals.fact_count, totals.total_points
    );
    if panel.recent.is_empty() {
        text.push_str("No achievements earned this session.\r\n");
    }
    for notice in &panel.recent {
        text.push_str(&format!("Achievement earned: {}\r\n", notice.name));
    }
    text
}
