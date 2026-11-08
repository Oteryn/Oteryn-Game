use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    format!(
        "Player reports\r\nGuided bug report: Ctrl+Shift+F12\r\nName: quote the name; statement: quote the original words; bot: describe repeated behavior and time. Optional translations are labelled unverified. Reason codes: 0–20, according to operator rules.\r\n{}\r\n{}\r\n{}",
        panel.violation_preview,
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
