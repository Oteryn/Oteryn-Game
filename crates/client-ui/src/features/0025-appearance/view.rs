use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    let mut text = format!(
        "Appearance\r\n{}\r\n{}",
        panel.summary,
        if panel.pending {
            "Waiting for the server"
        } else if !panel.selected && panel.feedback.is_empty() {
            "Unavailable in this session"
        } else {
            &panel.feedback
        }
    );
    if let Some(catalogue) = &panel.catalogue {
        text.push_str("\r\nOutfit gallery (server-owned catalogue)\r\n");
        for outfit in &catalogue.outfits {
            text.push_str(&format!(
                "{}: look {}; permitted addons {}\r\n",
                outfit.key, outfit.look_type, outfit.permitted_addons
            ));
        }
        if let Some(outfit) = catalogue
            .outfits
            .iter()
            .find(|outfit| outfit.key == panel.fields[0])
        {
            text.push_str(&format!(
                "Preview: {} (look {}), addons {}, colours {}, mount {}\r\n",
                outfit.key, outfit.look_type, panel.fields[1], panel.fields[3], panel.fields[2]
            ));
        }
    }
    text
}
