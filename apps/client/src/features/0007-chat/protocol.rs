use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn chat_initialize(&self, selected: bool) {
        lock(&self.0).chat.selected = selected;
    }
    pub(in crate::play) fn local_chat_result(&self, outcome: &oteryn_session::ChatOutcome) {
        oteryn_client_ui::chat::apply_result(&mut lock(&self.0).chat, outcome);
    }
}
