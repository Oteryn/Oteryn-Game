use super::ChatPartyRoomPanel;
#[must_use]
pub fn panel_text(panel: &ChatPartyRoomPanel) -> String {
    if !panel.selected {
        return "Party chat unavailable in this session".into();
    }
    let mut text = format!("Party chat — {} received lines", panel.lines.len());
    if let Some(feedback) = &panel.feedback {
        text.push_str("\r\n");
        text.push_str(feedback);
    }
    text
}
