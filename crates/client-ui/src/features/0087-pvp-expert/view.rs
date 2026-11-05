use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    format!(
        "PvP expert\r\n{}\r\n{}",
        panel.summary,
        if panel.pending {
            "Waiting for the server"
        } else if !panel.selected && panel.feedback.is_empty() {
            "Unavailable in this session"
        } else {
            &panel.feedback
        }
    )
}
