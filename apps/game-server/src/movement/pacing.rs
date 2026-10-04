//! SPEED-1 player step pacing (CONDITIONS-0 §4.3).
//!
//! One connection's pacing clock: a step may run once the previous moved step's duration has
//! passed. The connection keeps at most one early step in a one-step buffer until then; a second
//! request that arrives while the buffer is full is refused (`TOO_EARLY` under capability 13
//! `PACED_MOVEMENT_V1`, `Rejected` without it). The clock lives with the connection, so the
//! buffer is dropped at disconnect and a resumed connection starts with an empty one.

use std::time::Duration;
use tokio::time::Instant;

/// When the next step of one connection may run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct StepPacer {
    next_due: Option<Instant>,
}

impl StepPacer {
    /// `None` when a step requested at `now` may run at once, otherwise the instant it is due.
    pub(crate) fn wait_until(&self, now: Instant) -> Option<Instant> {
        self.next_due.filter(|due| now < *due)
    }

    /// A step ran at `ran_at`. Only a step that moved has a duration (`Some`); a blocked or
    /// refused step leaves the clock as it was.
    pub(crate) fn record(&mut self, ran_at: Instant, duration: Option<Duration>) {
        if let Some(duration) = duration {
            self.next_due = ran_at.checked_add(duration).or(self.next_due);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_waits_only_for_the_previous_moved_step() {
        let start = Instant::now();
        let mut pacer = StepPacer::default();
        assert_eq!(pacer.wait_until(start), None);
        pacer.record(start, Some(Duration::from_millis(550)));
        let due = start + Duration::from_millis(550);
        assert_eq!(pacer.wait_until(start), Some(due));
        assert_eq!(
            pacer.wait_until(start + Duration::from_millis(549)),
            Some(due)
        );
        assert_eq!(pacer.wait_until(due), None);
        // A blocked step leaves the clock unchanged.
        pacer.record(due, None);
        assert_eq!(pacer.wait_until(due), None);
        pacer.record(due, Some(Duration::from_millis(300)));
        assert_eq!(
            pacer.wait_until(due),
            Some(due + Duration::from_millis(300))
        );
    }
}
