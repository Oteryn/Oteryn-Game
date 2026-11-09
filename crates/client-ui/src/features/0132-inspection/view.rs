use super::*;
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return tr!("Inspection is unavailable in this session").into();
    }
    let mut text = lfmt!("Inspection\r\n{}", panel.feedback);
    if let Some(reply) = &panel.reply {
        for item in &reply.items {
            text.push_str(&lfmt!(
                "\r\nSlot {}: {} ×{} (tier {})",
                item.slot,
                item.name,
                item.count,
                item.tier
            ));
            for (key, value) in &item.attributes {
                text.push_str(&format!("\r\n  {key}: {value}"));
            }
        }
        text.push_str(&lfmt!("\r\nPermission requests: {}", reply.requests.len()));
    }
    text
}

/// Only public catalogue replies enter this browser; inspected players remain private.
pub fn catalogue_text(panel: &Panel) -> String {
    use std::fmt::Write;
    let mut text = String::from(tr!("Items — confirmed catalogue templates\r\n"));
    if !panel.selected {
        return tr!("Item catalogue unavailable in this session").into();
    }
    let _ = lwrite!(
        text,
        "Categories: All, {}\r\n{}\r\n",
        panel.categories().join(", "),
        panel.feedback
    );
    let search = panel.search.to_lowercase();
    let mut matched = 0;
    for item in panel.catalogue.values().filter(|item| {
        item.name.to_lowercase().contains(&search)
            && (panel.category.is_empty()
                || super::state::item_categories(item)
                    .any(|category| category.eq_ignore_ascii_case(&panel.category)))
    }) {
        matched += 1;
        let _ = lwrite!(
            text,
            "{}: {} (tier {})\r\n",
            item.definition_ref,
            item.name,
            item.tier
        );
        for (key, value) in &item.attributes {
            let _ = write!(text, "  {key}: {value}\r\n");
        }
    }
    if matched == 0 && !panel.catalogue.is_empty() {
        text.push_str(tr!("No items match the selected name and category.\r\n"));
    }
    if panel.catalogue.is_empty() {
        text.push_str(tr!(
            "Inspect a catalogue definition to load its statistics and sources.\r\n"
        ));
    }
    text
}
