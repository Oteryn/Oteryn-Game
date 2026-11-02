use crate::play::{Outbox, Pushed, mailbox::lock};
use oteryn_session::{AppliedDelta, ChatLine, ChatRoomSet};
impl Outbox {
    pub(in crate::play) fn chat_party_room_initialize(&self, selected: bool) {
        lock(&self.0).chat_party_room.selected = selected;
    }
}
pub(in crate::play) fn chat_line(pushed: &mut Pushed, delta: AppliedDelta<ChatLine>) {
    pushed.chat_lines.push(delta.value);
}
pub(in crate::play) fn chat_rooms(pushed: &mut Pushed, delta: AppliedDelta<ChatRoomSet>) {
    pushed.chat_rooms = Some(delta.value);
}
