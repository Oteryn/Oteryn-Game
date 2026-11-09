use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    let mut text = lfmt!(
        "Party shields and skulls\r\n{}\r\n{}",
        panel.summary,
        if panel.pending {
            tr!("Waiting for the server")
        } else if !panel.selected && panel.feedback.is_empty() {
            tr!("Unavailable in this session")
        } else {
            &panel.feedback
        }
    );
    if let Some(timer) = panel.skull_timer {
        text.push_str(&format!("\r\n{}", timer.text()));
    }
    if let Some(snapshot) = &panel.unjustified {
        text.push_str(&format!("\r\n{}", snapshot.text()));
    }
    text
}
