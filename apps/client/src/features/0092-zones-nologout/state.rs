//! Ordered messages use the session's existing event budget and fail closed on overflow.
use oteryn_session::{AppliedDelta, MAX_QUEUED_EVENTS, ServerMessage};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerMessageInbox {
    messages: Vec<AppliedDelta<ServerMessage>>,
    overflowed: bool,
}

impl ServerMessageInbox {
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    #[must_use]
    pub const fn overflowed(&self) -> bool {
        self.overflowed
    }

    pub fn iter(&self) -> impl Iterator<Item = &AppliedDelta<ServerMessage>> {
        self.messages.iter()
    }

    pub(in crate::play) fn push(&mut self, message: AppliedDelta<ServerMessage>) {
        if self.messages.len() == MAX_QUEUED_EVENTS {
            self.overflowed = true;
        } else {
            self.messages.push(message);
        }
    }

    pub(in crate::play) fn append(&mut self, newer: Self) {
        self.overflowed |= newer.overflowed;
        for message in newer.messages {
            self.push(message);
        }
    }
}
