//! Bounded per-owner debug flight recording.

use crate::{FieldValue, Level, Line, emit, unix_ms};
use std::collections::VecDeque;

/// Default number of retained flight entries.
pub const DEFAULT_CAPACITY: usize = 64;
/// Hard maximum number of retained flight entries.
pub const MAX_CAPACITY: usize = 1024;

/// A bounded ring of recent diagnostic lines which is independent of `OTERYN_LOG`.
#[derive(Debug)]
pub struct FlightRecorder {
    capacity: usize,
    entries: VecDeque<String>,
}

impl FlightRecorder {
    /// Create a recorder. Capacity is clamped to the hard maximum of 1024.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.min(MAX_CAPACITY),
            entries: VecDeque::with_capacity(capacity.min(MAX_CAPACITY)),
        }
    }

    /// Record a rendered line without consulting the global level filter.
    pub fn record(&mut self, line: &Line<'_>) {
        if self.capacity == 0 {
            return;
        }
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(line.render("flight", unix_ms()));
    }

    /// Emit the retained entries and a dump summary at warning level, then empty the ring.
    pub fn dump(&mut self, process: &str, trace: &str, reason: &str) {
        let count = self.entries.len();
        for (sequence, entry) in self.entries.drain(..).enumerate() {
            emit(
                process,
                &Line::new(Level::Warn, "flight", "flight")
                    .trace(trace)
                    .field("seq", FieldValue::U64(sequence as u64))
                    .detail(&entry),
            );
        }
        emit(
            process,
            &Line::new(Level::Warn, "flight", "flight_dump")
                .trace(trace)
                .field("reason", FieldValue::Token(reason))
                .field("entries", FieldValue::U64(count as u64)),
        );
    }

    /// Discard all retained entries without emitting them.
    pub fn discard(&mut self) {
        self.entries.clear();
    }

    /// Number of currently retained entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no entries are currently retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for FlightRecorder {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::{capture, parse_line};

    #[test]
    fn ring_is_bounded_and_dump_has_entries_then_summary() {
        let mut recorder = FlightRecorder::new(2);
        for event in ["one", "two", "three"] {
            recorder.record(&Line::new(Level::Debug, "test", event).detail("private\ntext"));
        }
        assert_eq!(recorder.len(), 2);
        let lines = capture(|| recorder.dump("p", "trace", "failed"));
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("event=flight") && lines[0].contains("seq=0"));
        assert!(lines[1].contains("seq=1"));
        assert!(
            lines[2].contains("event=flight_dump") && lines[2].contains("reason=failed entries=2")
        );
        assert!(
            parse_line(&lines[0])
                .unwrap()
                .detail
                .unwrap()
                .contains("\\n")
        );
        assert!(recorder.is_empty());
    }

    #[test]
    fn capacity_is_clamped_and_discard_is_silent() {
        let mut recorder = FlightRecorder::new(MAX_CAPACITY + 1);
        for _ in 0..MAX_CAPACITY + 5 {
            recorder.record(&Line::new(Level::Debug, "test", "entry"));
        }
        assert_eq!(recorder.len(), MAX_CAPACITY);
        recorder.discard();
        assert!(recorder.is_empty());
    }
}
