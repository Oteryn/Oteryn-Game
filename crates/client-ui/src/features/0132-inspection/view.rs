use super::*;
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Inspection is unavailable in this session".into();
    }
    let mut text = format!("Inspection\r\n{}", panel.feedback);
    if let Some(reply) = &panel.reply {
        for item in &reply.items {
            text.push_str(&format!(
                "\r\nSlot {}: {} ×{} (tier {})",
                item.slot, item.name, item.count, item.tier
            ));
            for (key, value) in &item.attributes {
                text.push_str(&format!("\r\n  {key}: {value}"));
            }
        }
        text.push_str(&format!(
            "\r\nPermission requests: {}",
            reply.requests.len()
        ));
    }
    text
}

/// Only public catalogue replies enter this browser; inspected players remain private.
pub fn catalogue_text(panel: &Panel) -> String {
    use std::fmt::Write;
    let mut text = String::from("Items — confirmed catalogue templates\r\n");
    if !panel.selected {
        return "Item catalogue unavailable in this session".into();
    }
    let search = panel.search.to_lowercase();
    for item in panel.catalogue.values().filter(|item| {
        item.name.to_lowercase().contains(&search)
            && (panel.category.is_empty()
                || item.attributes.iter().any(|(k, v)| {
                    (k.eq_ignore_ascii_case("category")
                        || k == "Weapon Type"
                        || k == "Body Position")
                        && v.split(',')
                            .next()
                            .is_some_and(|value| value.trim().eq_ignore_ascii_case(&panel.category))
                }))
    }) {
        let _ = write!(
            text,
            "{}: {} (tier {})\r\n",
            item.definition_ref, item.name, item.tier
        );
        for (key, value) in &item.attributes {
            let _ = write!(text, "  {key}: {value}\r\n");
        }
    }
    if panel.catalogue.is_empty() {
        text.push_str("Inspect a catalogue definition to load its statistics and sources.\r\n");
    }
    text
}
