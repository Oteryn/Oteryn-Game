use super::VipPresencePanel;
#[must_use]
pub fn panel_text(panel: &VipPresencePanel) -> String {
    if !panel.selected {
        return tr!("VIP presence unavailable in this session").into();
    }
    if panel.presence.entries.is_empty() {
        return tr!("No VIP presence updates").into();
    }
    panel
        .presence
        .entries
        .iter()
        .map(|entry| format!("{} — {:?}", entry.name, entry.status))
        .collect::<Vec<_>>()
        .join("\r\n")
}
