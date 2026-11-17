use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    format!(
        "Training\r\n{}\r\n{}\r\n{}",
        panel.batch_summary,
        panel.summary,
        if panel.pending {
            tr!("Waiting for the server")
        } else if !panel.selected && panel.feedback.is_empty() {
            tr!("Unavailable in this session")
        } else {
            &panel.feedback
        }
    )
}
