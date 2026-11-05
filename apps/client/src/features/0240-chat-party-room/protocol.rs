use crate::play::{Outbox, Pushed, mailbox::lock};
use oteryn_session::{AppliedDelta, ChatLine, ChatRoomSet};
impl Outbox {
    pub(in crate::play) fn chat_party_room_initialize(&self, selected: bool) {
        lock(&self.0).chat_party_room.selected = selected;
    }
    pub(in crate::play) fn chat_party_room_result(&self) {
        oteryn_client_ui::chat_party_room::apply_result(&mut lock(&self.0).chat_party_room);
    }
    pub(in crate::play) fn chat_result(
        &self,
        intent: &oteryn_session::ChatIntent,
        outcome: &oteryn_session::ChatOutcome,
    ) {
        if self.chat_private_result(intent, outcome) {
            return;
        }
        if matches!(intent, oteryn_session::ChatIntent::Party { .. }) {
            self.chat_party_room_result();
        } else if matches!(
            intent,
            oteryn_session::ChatIntent::Say { .. } | oteryn_session::ChatIntent::Private { .. }
        ) {
            self.local_chat_result(outcome);
        } else {
            self.chat_public_result(outcome);
        }
    }
}
pub(in crate::play) fn chat_line(pushed: &mut Pushed, delta: AppliedDelta<ChatLine>) {
    pushed.chat_lines.push(delta.value);
}
pub(in crate::play) fn chat_rooms(pushed: &mut Pushed, delta: AppliedDelta<ChatRoomSet>) {
    pushed.chat_rooms = Some(delta.value);
}
