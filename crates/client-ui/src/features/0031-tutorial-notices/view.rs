use super::NoticePanel;
#[must_use]
pub fn panel_text(panel: &NoticePanel) -> String {
    let mut text = String::from("Tutorial and server save (Ctrl+Shift+N)\r\n\r\n");
    if !panel.tutorial_selected {
        text.push_str("Tutorial notices unavailable on this server.\r\n");
    } else if panel.tutorial.is_empty() {
        text.push_str("No tutorial notices received.\r\n");
    }
    for line in &panel.tutorial {
        text.push_str(line);
        text.push_str("\r\n");
    }
    text.push_str("\r\n");
    if !panel.save_selected {
        text.push_str("Server-save notices unavailable on this server.");
    } else if let Some(minutes) = panel.save_minutes {
        text.push_str(&format!("Last server announcement: server save in {minutes} minute{}. Please find a safe place to log out.", if minutes == 1 { "" } else { "s" }));
    } else {
        text.push_str("No server-save announcement received.");
    }
    text.push_str("\r\n");
    text.push_str(&panel.events.text());
    text
}
