//! EVENTS-RAID-1 schedule metadata. Proposed facts are never admission, reset,
//! channel ownership, announcement delivery or creature realization authority.

mod calendar;
mod definition;
mod schedule;

pub use calendar::{CalendarDay, CalendarSelector};
pub use definition::{Announcement, AnnouncementKind, RaidDefinition, RaidPolicy, Wave, WaveEntry};
pub use schedule::{Firing, FiringState, RaidAction, RaidSchedule, RaidTransition};

pub const CHECK_INTERVAL_MS: i64 = 60_000;
pub const MAX_CANDIDATES: usize = 100;
pub const MAX_CREATURES: u16 = 200;
pub const MAX_WAVES: usize = 32;
pub const MAX_ANNOUNCEMENTS: usize = 16;
pub const MAX_ANNOUNCEMENT_BYTES: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidError {
    InvalidDefinition,
    InvalidState,
    InvalidTime,
    InvalidCalendar,
    Exhausted,
}

impl std::fmt::Display for RaidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "raid schedule: {self:?}")
    }
}
impl std::error::Error for RaidError {}

#[cfg(test)]
mod definition_tests;
#[cfg(test)]
mod schedule_tests;
