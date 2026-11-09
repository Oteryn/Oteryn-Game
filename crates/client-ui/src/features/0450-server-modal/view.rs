use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    match &panel.active {
        Some(dialog) => format!(
            "{}\r\n{}\r\n{}",
            dialog.title,
            dialog.message,
            panel.feedback.unwrap_or("")
        ),
        None => panel.feedback.unwrap_or("No server dialog pending").into(),
    }
}
