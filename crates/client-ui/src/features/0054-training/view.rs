use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    format!(
        "Training\r\n{}\r\n{}\r\n{}",
        panel.batch_summary,
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
