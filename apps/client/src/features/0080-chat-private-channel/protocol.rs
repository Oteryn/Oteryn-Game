use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_session::{ChatIntent, ChatOutcome};
impl Outbox {
    pub(in crate::play) fn chat_private_publish<T: Stepper>(&self, session: &T) {
        let (selected, list) = session.chat_private_state();
        let mut mailbox = lock(&self.0);
        mailbox.chat_private.selected = selected;
        let lines = session
            .chat_state()
            .map_or_else(Vec::new, |(_, lines)| lines);
        oteryn_client_ui::chat_private_channel::replace_state(
            &mut mailbox.chat_private,
            list,
            lines,
        );
    }
    pub(in crate::play) fn chat_private_result(
        &self,
        intent: &ChatIntent,
        outcome: &ChatOutcome,
    ) -> bool {
        if !matches!(intent, ChatIntent::PrivateChannel(_)) {
            return false;
        }
        oteryn_client_ui::chat_private_channel::apply_result(
            &mut lock(&self.0).chat_private,
            outcome,
        );
        true
    }
}
