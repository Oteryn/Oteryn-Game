use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    let mut text = String::from(tr!("Rotate house furniture\r\n"));
    if let Some(catalogue) = &panel.catalogue {
        text.push_str(&lfmt!("Page {}\r\n", catalogue.page + 1));
        for (index, entry) in catalogue.entries.iter().enumerate() {
            text.push_str(&lfmt!("{}. {} — Rotate\r\n", index + 1, entry.label));
        }
        if catalogue.entries.is_empty() {
            text.push_str(tr!("No nearby furniture to rotate\r\n"));
        }
    }
    text.push_str(if panel.pending {
        tr!("Waiting for the server")
    } else if !panel.selected {
        tr!("Unavailable in this session")
    } else {
        &panel.feedback
    });
    text
}
