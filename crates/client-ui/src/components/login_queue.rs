//! Queue presentation uses only a position supplied by the admission authority.
use std::time::{Duration, Instant};

/// One cancellable wait; polling cannot submit duplicate retries.
#[derive(Debug, Clone)]
pub struct QueueWait {
    position: Option<u32>,
    started: Instant,
    delay: Duration,
    active: bool,
}
impl QueueWait {
    pub fn new(position: Option<u32>, started: Instant, delay: Duration) -> Self {
        Self {
            position,
            started,
            delay,
            active: true,
        }
    }
    pub fn text(&self, now: Instant) -> String {
        if !self.active {
            return tr!("Queue wait ended.").into();
        }
        let seconds = self.remaining_seconds(now);
        let position = self.position.map_or_else(
            || tr!("Waiting for capacity.").into(),
            |n| lfmt!("Queue position {n}.", n = n),
        );
        lfmt!(
            "{position} Retrying in {seconds}s.",
            position = position,
            seconds = seconds
        )
    }
    pub fn remaining_seconds(&self, now: Instant) -> u64 {
        let remaining = self
            .delay
            .saturating_sub(now.saturating_duration_since(self.started));
        remaining.as_secs() + u64::from(remaining.subsec_nanos() != 0)
    }
    pub fn cancel(&mut self) {
        self.active = false;
    }
    pub fn take_retry(&mut self, now: Instant) -> bool {
        if !self.active || now.saturating_duration_since(self.started) < self.delay {
            return false;
        }
        self.active = false;
        true
    }
}
