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
        let list = mailbox.chat_private.list.clone();
        mailbox.chat.console.sync_private(list.as_ref());
    }
    pub(in crate::play) fn chat_private_result(
        &self,
        intent: &ChatIntent,
        outcome: &ChatOutcome,
    ) -> bool {
        if !matches!(intent, ChatIntent::PrivateChannel(_)) {
            return false;
        }
        let mut mailbox = lock(&self.0);
        let (pending, channel) = (
            mailbox.chat_private.pending_intent,
            mailbox.chat_private.channel,
        );
        oteryn_client_ui::chat_private_channel::apply_result(&mut mailbox.chat_private, outcome);
        oteryn_client_ui::chat::private_result(&mut mailbox.chat, pending, channel, outcome);
        // A line sent from a private-channel tab of the console ends its pending state too.
        if mailbox.chat.pending {
            oteryn_client_ui::chat::apply_result(&mut mailbox.chat, outcome);
        }
        true
    }
}
