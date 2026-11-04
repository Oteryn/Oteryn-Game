use super::AchievementNotices;

#[must_use]
pub fn panel_text(notices: &AchievementNotices) -> String {
    if !notices.selected {
        return "Achievement notices unavailable in this session".into();
    }
    if notices.advancement_log().len() == 0 {
        return "No achievements earned this session".into();
    }
    notices.advancement_log().collect::<Vec<_>>().join("\r\n")
}

#[must_use]
pub fn popup_text(notices: &AchievementNotices) -> Option<String> {
    notices.popup().map(|popup| popup.text.clone())
}
