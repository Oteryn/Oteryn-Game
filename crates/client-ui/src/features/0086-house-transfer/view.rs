use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    lfmt!(
        "House transfer\r\n{}\r\n{}",
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
