use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    let mut text = format!(
        "Party shields and skulls\r\n{}\r\n{}",
        panel.summary,
        if panel.pending {
            "Waiting for the server"
        } else if !panel.selected && panel.feedback.is_empty() {
            "Unavailable in this session"
        } else {
            &panel.feedback
        }
    );
    if let Some(timer) = panel.skull_timer {
        text.push_str(&format!("\r\n{}", timer.text()));
    }
    text
}
