//! The per-session chat egress queue (CHAT-0 §7, `CHAT0-RL-11`).
//!
//! At most [`EGRESS_MAX_LINES`] undelivered lines wait per session. Beyond it the oldest line of
//! the cheapest class is dropped (room, then local, then private) and one
//! [`ChatLine::Dropped`] marker is owed; the marker takes no slot and is handed out once, ahead
//! of the next line. Overflow after it re-arms nothing until a line has been sent, so a flood
//! can neither grow a session's backlog nor starve its queued lines. A batch handed out by
//! [`ChatEgress::drain_batch`] stays counted against the bound until each is [`ChatEgress::delivered`],
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
    /// A marker was sent and no line has been sent since: further overflow stays covered by it.
    marker_sent: bool,
    /// Lines handed out by [`Self::drain_batch`] and not yet written to the client.
    in_flight: usize,
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
        if self.lines.len() + self.in_flight >= EGRESS_MAX_LINES {
            self.marker_owed = !self.marker_sent;
            let worst = self.lines.iter().map(drop_rank).max().unwrap_or(0);
            if drop_rank(&line) > worst {
                return;
            }
            if let Some(index) = self.lines.iter().position(|q| drop_rank(q) == worst) {
                self.lines.remove(index);
            }
        }
        self.lines.push_back(line);
    }

    /// The next line to send: the owed marker first, then the oldest line.
    pub(crate) fn pop(&mut self) -> Option<ChatLine> {
        if std::mem::take(&mut self.marker_owed) {
            self.marker_sent = true;
            return Some(ChatLine::Dropped);
        }
        let line = self.lines.pop_front();
        if line.is_some() {
            self.marker_sent = false;
        }
        line
    }

    /// Hands out every queued line (the owed marker first). The lines count against the bound
    /// until [`Self::delivered`].
    pub(crate) fn drain_batch(&mut self) -> Vec<ChatLine> {
        let batch: Vec<ChatLine> = std::iter::from_fn(|| self.pop()).collect();
        self.in_flight = batch
            .iter()
            .filter(|line| !matches!(line, ChatLine::Dropped))
            .count();
        batch
    }

    /// `lines` lines of the last [`Self::drain_batch`] were written to the client.
    pub(crate) fn delivered(&mut self, lines: usize) {
        self.in_flight = self.in_flight.saturating_sub(lines);
    }

    /// Drops every queued line without a marker (a replacement snapshot drops lines).
    pub(crate) fn clear(&mut self) {
        self.in_flight = 0;
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
