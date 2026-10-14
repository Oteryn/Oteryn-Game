//! The per-session chat egress queue (CHAT-0 §7, `CHAT0-RL-11`).
//!
//! At most [`EGRESS_MAX_LINES`] undelivered lines wait per session. Beyond it the oldest line of
//! the cheapest class is dropped (room, then local, then private) and one
//! [`ChatLine::Dropped`] marker is owed; the marker takes no slot and is handed out once, ahead
//! of the next line. Overflow after it re-arms nothing until a line has been sent, so a flood
//! can neither grow a session's backlog nor starve its queued lines. The one line handed out by
//! [`ChatEgress::next_for_write`] stays counted against the bound until [`ChatEgress::delivered`],
//! so queued plus in-flight lines never exceed [`EGRESS_MAX_LINES`].

use std::collections::VecDeque;

use oteryn_protocol_oteryn::chat::ChatLine;

/// `CHAT0-RL-11`: undelivered lines per session.
pub(crate) const EGRESS_MAX_LINES: usize = 64;

/// Drop order: a higher rank is dropped first.
fn drop_rank(line: &ChatLine) -> u8 {
    match line {
        ChatLine::Room { .. } => 3,
        ChatLine::Local { .. } => 2,
        ChatLine::Private { .. } | ChatLine::Dropped => 1,
    }
}

/// One session's undelivered chat lines.
#[derive(Debug, Default)]
pub(crate) struct ChatEgress {
    lines: VecDeque<ChatLine>,
    marker_owed: bool,
    /// A marker was sent and no line has been delivered since: further overflow stays covered
    /// by it.
    marker_sent: bool,
    /// Drop rank of the line handed out by [`Self::next_for_write`] and not yet delivered; 0
    /// when the slot is free (nothing in flight, or the line was retracted from the count).
    in_flight_rank: u8,
    /// A real line is being written to the client.
    writing: bool,
}

impl ChatEgress {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Queues `line`; a [`ChatLine::Dropped`] argument only owes the marker.
    pub(crate) fn push(&mut self, line: ChatLine) {
        if matches!(line, ChatLine::Dropped) {
            self.marker_owed = true;
            return;
        }
        let slot = usize::from(self.in_flight_rank > 0);
        if self.lines.len() + slot >= EGRESS_MAX_LINES {
            let queued_worst = self.lines.iter().map(drop_rank).max().unwrap_or(0);
            let worst = queued_worst.max(self.in_flight_rank);
            if drop_rank(&line) > worst {
                self.marker_owed = !self.marker_sent;
                return;
            }
            if self.in_flight_rank == worst {
                // The in-flight line is the oldest of the cheapest class. It is already being
                // written and still reaches the client, so it is retracted from the count only:
                // nothing is lost and no marker is owed.
                self.in_flight_rank = 0;
            } else {
                self.marker_owed = !self.marker_sent;
                if let Some(index) = self.lines.iter().position(|q| drop_rank(q) == worst) {
                    self.lines.remove(index);
                }
            }
        }
        self.lines.push_back(line);
    }

    /// The next line to send: the owed marker first, then the oldest line.
    pub(crate) fn pop(&mut self) -> Option<ChatLine> {
        let line = self.take()?;
        if !matches!(line, ChatLine::Dropped) {
            self.marker_sent = false;
        }
        Some(line)
    }

    fn take(&mut self) -> Option<ChatLine> {
        if std::mem::take(&mut self.marker_owed) {
            self.marker_sent = true;
            return Some(ChatLine::Dropped);
        }
        self.lines.pop_front()
    }

    /// Hands out the next line to write (the owed marker first). A real line counts against the
    /// bound until [`Self::delivered`]; one line at a time keeps every other undelivered line
    /// queued and so subject to the drop order.
    pub(crate) fn next_for_write(&mut self) -> Option<ChatLine> {
        let line = self.take()?;
        let real = !matches!(line, ChatLine::Dropped);
        self.writing = real;
        self.in_flight_rank = if real { drop_rank(&line) } else { 0 };
        Some(line)
    }

    /// The last real line of [`Self::next_for_write`] was written to the client: its slot is
    /// free and a marker no longer covers later overflow.
    pub(crate) fn delivered(&mut self) {
        if std::mem::take(&mut self.writing) {
            self.marker_sent = false;
        }
        self.in_flight_rank = 0;
    }

    /// Drops every queued line without a marker (a replacement snapshot drops lines).
    pub(crate) fn clear(&mut self) {
        self.in_flight_rank = 0;
        self.writing = false;
        self.lines.clear();
        self.marker_owed = false;
        self.marker_sent = false;
    }

    pub(crate) fn len(&self) -> usize {
        self.lines.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lines.is_empty() && !self.marker_owed
    }
}
