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
pub(crate) trait PacingTime: Copy + Ord {
    fn checked_after(self, duration: Duration) -> Option<Self>;
}
impl PacingTime for Instant {
    fn checked_after(self, duration: Duration) -> Option<Self> {
        self.checked_add(duration)
    }
}
impl PacingTime for oteryn_simulation_determinism::SemanticTimeMicros {
    fn checked_after(self, duration: Duration) -> Option<Self> {
        let micros = u64::try_from(duration.as_micros()).ok()?;
        self.get().checked_add(micros).map(Self::from_micros)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StepPacer<T = Instant> {
    next_due: Option<T>,
}
impl<T> Default for StepPacer<T> {
    fn default() -> Self {
        Self { next_due: None }
    }
}
/// Same moved-only pacing kernel, driven by the independently supplied Channel owner clock.
pub(crate) type CreatureStepPacer = StepPacer<oteryn_simulation_determinism::SemanticTimeMicros>;

impl<T: PacingTime> StepPacer<T> {
    /// `None` when a step requested at `now` may run at once, otherwise the instant it is due.
    pub(crate) fn wait_until(&self, now: T) -> Option<T> {
        self.next_due.filter(|due| now < *due)
    }

    /// Preflight overflow and the current deadline BEFORE a Movement mutation.
    pub(crate) fn prepare_moved_deadline(&self, now: T, duration: Duration) -> Option<T> {
        if self.wait_until(now).is_some() {
            return None;
        }
        now.checked_after(duration)
    }

    /// A step ran at `ran_at`. Only a step that moved has a duration (`Some`); a blocked or
    /// refused step leaves the clock as it was.
    pub(crate) fn record(&mut self, ran_at: T, duration: Option<Duration>) {
        if let Some(duration) = duration {
            self.next_due = ran_at.checked_after(duration).or(self.next_due);
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

#[cfg(test)]
mod creature_semantic_pacing_tests {
    use super::*;
    use oteryn_simulation_determinism::SemanticTimeMicros as Time;
    #[test]
    fn canonical_creature_pacer_preserves_moved_only_deadline_and_beat_boundary() {
        let mut pacer = CreatureStepPacer::default();
        let now = Time::from_micros(100);
        let duration = crate::movement::speed::StepSpeedTable::embedded()
            .unwrap()
            .step_duration(220, 150)
            .unwrap();
        assert_eq!(duration, Duration::from_millis(300));
        assert_eq!(
            pacer.prepare_moved_deadline(now, duration),
            Some(Time::from_micros(300_100))
        );
        // Refused/blocked Movement does not install the preflight deadline.
        pacer.record(now, None);
        assert_eq!(pacer.wait_until(now), None);
        pacer.record(now, Some(duration));
        assert_eq!(
            pacer.wait_until(Time::from_micros(300_099)),
            Some(Time::from_micros(300_100))
        );
        assert_eq!(
            pacer.prepare_moved_deadline(Time::from_micros(300_099), duration),
            None
        );
        assert_eq!(pacer.wait_until(Time::from_micros(300_100)), None);
        pacer.record(Time::from_micros(300_100), None);
        assert_eq!(
            pacer.prepare_moved_deadline(Time::from_micros(300_100), duration),
            Some(Time::from_micros(600_100))
        );
    }
    #[test]
    fn creature_cadence_overflow_is_preflight_refusal_with_unchanged_pacer() {
        let pacer = CreatureStepPacer::default();
        assert_eq!(
            pacer.prepare_moved_deadline(Time::from_micros(u64::MAX), Duration::from_micros(1)),
            None
        );
        assert_eq!(pacer.wait_until(Time::from_micros(u64::MAX)), None);
        assert_eq!(
            pacer.prepare_moved_deadline(Time::from_micros(0), Duration::MAX),
            None
        );
    }
}
