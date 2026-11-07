use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// SPECTATE1-RL-04: rolling, not calendar/fixed, and at most five retained timestamps.
#[derive(Default)]
pub(super) struct RateWindow(VecDeque<Instant>);
impl RateWindow {
    pub(super) fn expire(&mut self, now: Instant) -> bool {
        while self
            .0
            .front()
            .is_some_and(|time| now.saturating_duration_since(*time) >= Duration::from_secs(60))
        {
            self.0.pop_front();
        }
        self.0.is_empty()
    }
    pub(super) fn attempt(&mut self, now: Instant) -> bool {
        self.expire(now);
        if self.0.len() == 5 {
            return false;
        }
        self.0.push_back(now);
        true
    }
}

#[cfg(test)]
#[path = "rate_window_tests.rs"]
mod tests;
