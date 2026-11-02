use crate::play::{PlayView, ServerMessageInbox};
use oteryn_session::{AppliedDelta, MAX_QUEUED_EVENTS, ServerMessage};

impl PlayView {
    pub(in crate::play) fn apply_server_messages(&mut self, messages: &ServerMessageInbox) {
        for message in messages.iter() {
            if self.server_messages.len() == MAX_QUEUED_EVENTS {
                self.server_messages.pop_front();
            }
            self.server_messages.push_back(message.clone());
        }
    }

    /// Recent notices in delivery order, reset with the admitted play view on reconnect.
    pub fn server_messages(&self) -> impl Iterator<Item = &AppliedDelta<ServerMessage>> {
        self.server_messages.iter()
    }

    /// The existing native status surface displays the latest notice without erasing combat.
    #[must_use]
    pub fn server_message_status(&self) -> Option<String> {
        self.server_messages
            .back()
            .map(|message| match &message.value {
                ServerMessage::StatusText(text) => text.clone(),
                ServerMessage::Warning(text) => format!("Warning: {text}"),
                ServerMessage::Advance(text) => format!("Advance: {text}"),
            })
    }
}
