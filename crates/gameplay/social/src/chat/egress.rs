//! The per-session chat egress queue (CHAT-0 §7, `CHAT0-RL-11`).
//!
//! At most [`EGRESS_MAX_LINES`] undelivered lines wait per session. Beyond it the oldest line of
//! the cheapest class is dropped (room, then local, then private) and one
//! [`ChatLine::Dropped`] marker is owed; the marker takes no slot and is handed out once, ahead
//! of the next line. Overflow after it re-arms nothing until a line has been delivered, so a flood
//! can neither grow a session's backlog nor starve its queued lines. The one line handed out by
//! [`ChatEgress::next_for_write`] stays counted against the bound until [`ChatEgress::delivered`],
//! so queued plus in-flight lines never exceed [`EGRESS_MAX_LINES`]. The in-flight line is already
//! on its way to the client and cannot be retracted, so the drop order covers the queued lines only.

use std::collections::VecDeque;

use oteryn_protocol_oteryn::chat::ChatLine;

pub use oteryn_gameplay_social_rules::egress::EGRESS_MAX_LINES;
use oteryn_gameplay_social_rules::egress::{self, LineClass};

/// Translate a wire line into its pure drop class.
fn drop_rank(line: &ChatLine) -> u8 {
    egress::drop_rank(match line {
        ChatLine::Party { .. }
        | ChatLine::Guild(_)
        | ChatLine::PrivateChannel(_)
        | ChatLine::Room { .. } => LineClass::Room,
        ChatLine::Local { .. } => LineClass::Local,
        ChatLine::Private { .. } => LineClass::Private,
        ChatLine::Dropped => LineClass::Dropped,
    })
}

#[cfg(test)]
mod private_channel_queue_tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn private(
        epoch: u64,
    ) -> (
        oteryn_protocol_oteryn::chat::private_channel::PrivateChannelLine,
        PrivateChannelChatDestination,
    ) {
        (
            oteryn_protocol_oteryn::chat::private_channel::PrivateChannelLine {
                channel_id: 100,
                speaker_name: "Sender".into(),
                text: "private channel".into(),
            },
            PrivateChannelChatDestination {
                channel_id: 100,
                epoch,
                sent_at_micros: 1,
            },
        )
    }

    #[test]
    fn private_channel_lines_share_existing_queue_capacity_and_preserve_epoch() {
        let mut queue = ChatEgress::default();
        for _ in 0..EGRESS_MAX_LINES {
            queue.push(ChatLine::Room {
                room: oteryn_protocol_oteryn::chat::ChatRoom::World,
                speaker_name: "Sender".into(),
                text: "room".into(),
            });
        }
        for epoch in 1..=EGRESS_MAX_LINES as u64 {
            let (line, destination) = private(epoch);
            queue.push_private_channel(line, destination);
            assert!(queue.len() + usize::from(queue.in_flight) <= EGRESS_MAX_LINES);
        }
        assert_eq!(queue.next_for_write(), Some(ChatLine::Dropped));
        let destination = queue
            .private_channel_destination()
            .expect("first private metadata");
        assert_eq!(destination.epoch, 1);
        assert!(matches!(
            queue.next_for_write(),
            Some(ChatLine::PrivateChannel(_))
        ));
        let (line, destination) = private(99);
        queue.push_private_channel(line, destination);
        assert_eq!(queue.len() + usize::from(queue.in_flight), EGRESS_MAX_LINES);
        queue.delivered();
        assert!(queue.private_channel_destination().is_some());
        queue.clear();
        assert!(queue.private_channel_destination().is_none());
        assert_eq!(queue.len(), 0);
    }
}

/// Server-only routing evidence, retained with a queued Party line and never encoded.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PartyChatDestination {
    pub party_id: [u8; 16],
    pub sent_at_micros: u64,
}
impl std::fmt::Debug for PartyChatDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartyChatDestination")
            .finish_non_exhaustive()
    }
}

/// Server-only custody of one private-channel incarnation. It shares the existing queue bound.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PrivateChannelChatDestination {
    pub channel_id: u16,
    pub epoch: u64,
    pub sent_at_micros: u64,
}
impl std::fmt::Debug for PrivateChannelChatDestination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrivateChannelChatDestination")
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct QueuedLine {
    line: ChatLine,
    party: Option<PartyChatDestination>,
    private_channel: Option<PrivateChannelChatDestination>,
}

/// One session's undelivered chat lines.
#[derive(Debug, Default)]
pub struct ChatEgress {
    lines: VecDeque<QueuedLine>,
    marker_owed: bool,
    /// A marker was sent and no line has been delivered since: further overflow stays covered
    /// by it.
    marker_sent: bool,
    /// A real line was handed out by [`Self::next_for_write`] and is not yet delivered. It is
    /// being written to the client, so it cannot be retracted: it keeps its slot until then.
    in_flight: bool,
}

impl ChatEgress {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `line`; a [`ChatLine::Dropped`] argument only owes the marker.
    pub fn push(&mut self, line: ChatLine) {
        self.push_queued(QueuedLine {
            line,
            party: None,
            private_channel: None,
        });
    }

    pub fn push_party(
        &mut self,
        speaker_name: String,
        text: String,
        destination: PartyChatDestination,
    ) {
        self.push_queued(QueuedLine {
            line: ChatLine::Party { speaker_name, text },
            party: Some(destination),
            private_channel: None,
        });
    }

    pub fn push_private_channel(
        &mut self,
        line: oteryn_protocol_oteryn::chat::private_channel::PrivateChannelLine,
        destination: PrivateChannelChatDestination,
    ) {
        self.push_queued(QueuedLine {
            line: ChatLine::PrivateChannel(line),
            party: None,
            private_channel: Some(destination),
        });
    }

    pub fn private_channel_destination(&self) -> Option<PrivateChannelChatDestination> {
        if self.marker_owed {
            return None;
        }
        self.lines.front()?.private_channel
    }

    pub fn party_destination(&self) -> Option<PartyChatDestination> {
        if self.marker_owed {
            return None;
        }
        self.lines.front()?.party
    }

    fn push_queued(&mut self, line: QueuedLine) {
        if matches!(line.line, ChatLine::Dropped) {
            self.marker_owed = true;
            return;
        }
        if self.lines.len() + usize::from(self.in_flight) >= EGRESS_MAX_LINES {
            self.marker_owed = !self.marker_sent;
            let worst = self
                .lines
                .iter()
                .map(|queued| drop_rank(&queued.line))
                .max()
                .unwrap_or(0);
            if drop_rank(&line.line) > worst {
                return;
            }
            if let Some(index) = self.lines.iter().position(|q| drop_rank(&q.line) == worst) {
                self.lines.remove(index);
            }
        }
        self.lines.push_back(line);
    }

    /// Hands out the next line and acknowledges it as delivered at once (a write that cannot
    /// stall). Marker coverage is reset only through [`Self::delivered`].
    #[cfg(test)]
    pub fn pop(&mut self) -> Option<ChatLine> {
        let line = self.next_for_write()?;
        self.delivered();
        Some(line)
    }

    pub fn peek(&self) -> Option<&ChatLine> {
        if self.marker_owed {
            Some(&ChatLine::Dropped)
        } else {
            self.lines.front().map(|queued| &queued.line)
        }
    }

    fn take(&mut self) -> Option<ChatLine> {
        if std::mem::take(&mut self.marker_owed) {
            self.marker_sent = true;
            return Some(ChatLine::Dropped);
        }
        self.lines.pop_front().map(|queued| queued.line)
    }

    /// Hands out the next line to write (the owed marker first). A real line counts against the
    /// bound until [`Self::delivered`]; one line at a time keeps every other undelivered line
    /// queued and so subject to the drop order.
    pub fn next_for_write(&mut self) -> Option<ChatLine> {
        let line = self.take()?;
        self.in_flight = !matches!(line, ChatLine::Dropped);
        Some(line)
    }

    /// The last real line of [`Self::next_for_write`] was written to the client: its slot is
    /// free and a marker no longer covers later overflow.
    pub fn delivered(&mut self) {
        if std::mem::take(&mut self.in_flight) {
            self.marker_sent = false;
        }
    }

    /// Drops every queued line without a marker (a replacement snapshot drops lines).
    pub fn clear(&mut self) {
        self.in_flight = false;
        self.lines.clear();
        self.marker_owed = false;
        self.marker_sent = false;
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty() && !self.marker_owed
    }
}
