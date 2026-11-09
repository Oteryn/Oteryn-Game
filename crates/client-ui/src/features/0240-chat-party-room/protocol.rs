use super::ChatPartyRoomPanel;
use oteryn_client_runtime::chat_public_channels::ChatIntent;

pub fn request(panel: &mut ChatPartyRoomPanel) -> Option<ChatIntent> {
    if !panel.selected {
        panel.feedback = Some(tr!("Party chat is unavailable in this session").into());
        return None;
    }
    if panel.pending {
        panel.feedback = Some(tr!("Waiting for the server").into());
        return None;
    }
    if panel.text.is_empty() || panel.text.len() > 1_020 {
        panel.feedback = Some(tr!("Enter a message of at most 1,020 bytes").into());
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(ChatIntent::Party {
        text: panel.text.clone(),
    })
}

pub fn apply_result(panel: &mut ChatPartyRoomPanel) {
    panel.pending = false;
}

pub fn replace_rooms(
    panel: &mut ChatPartyRoomPanel,
    rooms: oteryn_client_runtime::cl25a_protocol::ChatRoomSet,
) {
    panel.rooms = rooms;
}
pub fn push_line(
    panel: &mut ChatPartyRoomPanel,
    line: oteryn_client_runtime::cl25a_protocol::ChatLine,
) {
    if !matches!(
        line,
        oteryn_client_runtime::cl25a_protocol::ChatLine::Party { .. }
    ) {
        return;
    }
    panel.lines.push(line);
    if panel.lines.len() > 100 {
        panel.lines.remove(0);
    }
}
