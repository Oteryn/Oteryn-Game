use super::ChatPartyRoomPanel;
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
