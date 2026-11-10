use super::*;
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return tr!("Quick loot is unavailable in this session").into();
    }
    let mut text = lfmt!(
        "Quick loot\r\n{}\r\nConfirmed changes this session (saved settings cannot be read back):",
        panel.feedback
    );
    for action in &panel.confirmed {
        text.push_str(&format!("\r\n{action:?}"));
    }
    text.push_str(tr!("\r\nManaged destinations (confirmed in this session):"));
    for destination in &panel.destinations {
        text.push_str(&format!("\r\n{destination:?}"));
    }
    if let Some(result) = &panel.result {
        text.push_str(&lfmt!(
            "\r\nLooted {} of {} items",
            result.items.iter().filter(|item| item.accepted).count(),
            result.items.len()
        ));
    }
    text
}
