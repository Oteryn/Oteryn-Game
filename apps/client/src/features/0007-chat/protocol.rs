use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn chat_initialize(&self, selected: bool) {
        lock(&self.0).chat.selected = selected;
    }
    pub(in crate::play) fn local_chat_result(&self, outcome: &oteryn_session::ChatOutcome) {
        oteryn_client_ui::chat::apply_result(&mut lock(&self.0).chat, outcome);
    }
}

/// Seeds server-owned chat state for shared frontend caller tests only.
#[cfg(all(test, feature = "bevy-shell"))]
pub(crate) fn bevy_chat_fixture(
    outbox: &Outbox,
    rooms: oteryn_session::ChatRoomSet,
    lines: Vec<oteryn_session::ChatLine>,
) {
    outbox.chat_initialize(true);
    outbox.chat_public_state(rooms, lines);
}
