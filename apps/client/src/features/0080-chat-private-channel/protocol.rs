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
    /// A new session counts its sequences and notices from the start.
    pub(in crate::play) fn chat_extras_reset(&self) {
        lock(&self.0).chat.console.reset_extensions();
    }
    /// Hands the console the extensions of the chat capabilities the session selected.
    pub(in crate::play) fn chat_extras_publish<T: Stepper>(&self, session: &T) {
        let extras = session.chat_extras();
        let mut mailbox = lock(&self.0);
        let own = mailbox.party.own_character_name.clone();
        let list = mailbox.chat_private.list.clone();
        mailbox.chat.apply_extras(&own, list.as_ref(), extras);
    }
    pub(in crate::play) fn chat_private_result(
        &self,
        intent: &ChatIntent,
        outcome: &ChatOutcome,
    ) -> bool {
        use oteryn_client_runtime::chat_private_channel::PrivateChannelIntent;
        use oteryn_client_ui::chat_private_channel::Intent;
        let ChatIntent::PrivateChannel(command) = intent else {
            return false;
        };
        let (pending, channel) = match command {
            PrivateChannelIntent::Join { channel_id } => (Some(Intent::Join), Some(*channel_id)),
            PrivateChannelIntent::Leave { channel_id } => (Some(Intent::Leave), Some(*channel_id)),
            _ => (None, None),
        };
        let mut mailbox = lock(&self.0);
        oteryn_client_ui::chat_private_channel::apply_result(&mut mailbox.chat_private, outcome);
        oteryn_client_ui::chat::private_result(&mut mailbox.chat, pending, channel, outcome);
        // A line sent from a private-channel tab of the console ends its pending state too.
        if mailbox.chat.pending && matches!(command, PrivateChannelIntent::Say { .. }) {
            oteryn_client_ui::chat::apply_result(&mut mailbox.chat, outcome);
        }
        true
    }
}
