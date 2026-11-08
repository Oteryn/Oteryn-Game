use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    let item = panel.preview.as_ref().map_or_else(
        || "Refresh to load its exact definition".to_owned(),
        |p| format!("{} @ {}", p.item_definition_key, p.item_definition_revision),
    );
    format!(
        "House podium\r\n{}\r\nPodium item: {}\r\nDraft visibility: display {}, platform {}. Use Preview to validate, Save to commit.\r\n{}",
        panel.summary,
        item,
        panel.visibility.display,
        panel.visibility.platform,
        if panel.pending {
            "Waiting for the server"
        } else if !panel.selected && panel.feedback.is_empty() {
            "Unavailable in this session"
        } else {
            &panel.feedback
        }
    )
}
