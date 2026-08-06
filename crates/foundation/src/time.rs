use std::fmt::{self, Display, Formatter};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Monotonic time elapsed from a clock-specific origin.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Moment(Duration);

impl Moment {
    /// The clock origin.
    pub const ZERO: Self = Self(Duration::ZERO);

    /// Construct a moment from elapsed monotonic time.
    #[must_use]
    pub const fn from_elapsed(elapsed: Duration) -> Self {
        Self(elapsed)
    }

    /// Return elapsed monotonic time from the clock origin.
    #[must_use]
    pub const fn elapsed(self) -> Duration {
        self.0
    }

    /// Add a duration without overflowing.
    pub fn checked_add(self, duration: Duration) -> Result<Self, TimeError> {
        self.0
            .checked_add(duration)
            .map(Self)
            .ok_or(TimeError::Overflow {
                base: self,
                duration,
            })
    }

    /// Measure a monotonic interval.
    pub fn checked_duration_since(self, start: Self) -> Result<Duration, TimeError> {
        self.0
            .checked_sub(start.0)
            .ok_or(TimeError::NonMonotonicInterval { start, end: self })
    }
}

impl Display for Moment {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write_duration(formatter, self.0)
    }
}

/// An absolute monotonic deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Deadline(Moment);

impl Deadline {
    #[must_use]
    pub const fn at(moment: Moment) -> Self {
        Self(moment)
    }

    pub fn after<C>(clock: &C, duration: Duration) -> Result<Self, TimeError>
    where
        C: MonotonicClock + ?Sized,
    {
        clock.now().checked_add(duration).map(Self)
    }

    #[must_use]
    pub const fn moment(self) -> Moment {
        self.0
    }

    #[must_use]
    pub fn has_elapsed<C>(self, clock: &C) -> bool
    where
        C: MonotonicClock + ?Sized,
    {
        clock.now() >= self.0
    }

    #[must_use]
    pub fn remaining<C>(self, clock: &C) -> Duration
    where
        C: MonotonicClock + ?Sized,
    {
        self.0.elapsed().saturating_sub(clock.now().elapsed())
    }
}

pub trait MonotonicClock: Send + Sync {
    fn now(&self) -> Moment;
}

#[derive(Debug, Clone)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for SystemClock {
    fn now(&self) -> Moment {
        Moment::from_elapsed(self.origin.elapsed())
    }
}

#[derive(Debug, Clone)]
pub struct ManualClock {
    current: Arc<RwLock<Moment>>,
}

impl ManualClock {
    #[must_use]
    pub fn new(start: Moment) -> Self {
        Self {
            current: Arc::new(RwLock::new(start)),
        }
    }

    pub fn advance(&self, duration: Duration) -> Result<Moment, TimeError> {
        let mut current = match self.current.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let next = current.checked_add(duration)?;
        *current = next;
        Ok(next)
    }

    pub fn try_set(&self, requested: Moment) -> Result<(), TimeError> {
        let mut current = match self.current.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if requested < *current {
            return Err(TimeError::BackwardMovement {
                current: *current,
                requested,
            });
        }
        *current = requested;
        Ok(())
    }
}

impl MonotonicClock for ManualClock {
    fn now(&self) -> Moment {
        let current = match self.current.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *current
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    Overflow { base: Moment, duration: Duration },
    BackwardMovement { current: Moment, requested: Moment },
    NonMonotonicInterval { start: Moment, end: Moment },
}

impl Display for TimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow { base, duration } => {
                write!(formatter, "monotonic time overflow at {base} while adding ")?;
                write_duration(formatter, *duration)
            }
            Self::BackwardMovement { current, requested } => write!(
                formatter,
                "manual clock cannot move backwards from {current} to {requested}"
            ),
            Self::NonMonotonicInterval { start, end } => write!(
                formatter,
                "monotonic interval end {end} is before start {start}"
            ),
        }
    }
}

impl std::error::Error for TimeError {}

fn write_duration(formatter: &mut Formatter<'_>, duration: Duration) -> fmt::Result {
    write!(
        formatter,
        "{}.{:09}s",
        duration.as_secs(),
        duration.subsec_nanos()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_and_deadline_are_deterministic() -> Result<(), TimeError> {
        let clock = ManualClock::new(Moment::ZERO);
        let deadline = Deadline::after(&clock, Duration::from_secs(3))?;
        clock.advance(Duration::from_secs(2))?;
        assert_eq!(deadline.remaining(&clock), Duration::from_secs(1));
        clock.advance(Duration::from_secs(1))?;
        assert!(deadline.has_elapsed(&clock));
        Ok(())
    }
}
