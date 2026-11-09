use super::ChatPartyRoomPanel;
#[must_use]
pub fn panel_text(panel: &ChatPartyRoomPanel) -> String {
    if !panel.selected {
        return tr!("Party chat unavailable in this session").into();
    }
    let mut text = String::from(tr!("Party chat"));
    for line in &panel.lines {
        if !panel.communication.visible(line) {
            continue;
        }
        if let oteryn_client_runtime::cl25a_protocol::ChatLine::Party {
            speaker_name,
            text: line,
        } = line
        {
            text.push_str(&format!("\r\n{speaker_name}: {line}"));
        }
    }
    if let Some(feedback) = &panel.feedback {
        text.push_str("\r\n");
        text.push_str(feedback);
    }
    text
}
