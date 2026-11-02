//! EVENTS-RAID-1 schedule metadata. Proposed facts are never admission, reset,
//! channel ownership, announcement delivery or creature realization authority.

mod calendar;
mod catalogue;
mod definition;
mod lifecycle;
mod lua_schedule;
mod placement;
mod runner;
mod schedule;

pub use calendar::{CalendarDay, CalendarSelector};
pub use catalogue::{
    CatalogueAmount, CatalogueAnnouncement, CatalogueAnnouncementKind, CatalogueArea, CatalogueEnd,
    CatalogueEntry, CatalogueKind, CataloguePosition, CatalogueRaid, CatalogueRegion,
    CatalogueSchedule, CatalogueSpawnMode, CatalogueVariant, CatalogueWave, CatalogueWindow,
    RaidCatalogue, SelectedRaid,
};
pub use definition::{Announcement, AnnouncementKind, RaidDefinition, RaidPolicy, Wave, WaveEntry};
pub use lifecycle::{LegacyRaidGate, RaidCalendarWindow, RaidEndDecision, RaidEndPolicy};
pub use lua_schedule::{
    LUA_RAID_ACTIVE_IDLE_MS, LUA_RAID_CHECKS_PER_DAY, LuaRaidCheck, LuaRaidPolicy, LuaRaidState,
    LuaRaidTransition, lua_raid_player_is_active, lua_raid_roll,
};
pub use placement::{RAID_PLACEMENT_ATTEMPTS, RaidArea, RaidPosition, RaidRandom, RaidTileFacts};
pub use runner::{RaidRunAction, RaidRunCursor, RaidRunStep};
pub use schedule::{Firing, FiringState, RaidAction, RaidSchedule, RaidTransition};

pub const CHECK_INTERVAL_MS: i64 = 60_000;
// RAIDS-GLOBAL 2026-11-01: 239 reconciled records. The bounded catalogue
// capacity covers the full source set, including explicitly inactive identities.
pub const MAX_CANDIDATES: usize = 512;
// Crystal 201189f yalahar/vampires.xml:6-12 and oramond/walkers.xml:4-53.
pub const MAX_CREATURES: u16 = 500;
pub const MAX_WAVES: usize = 50;
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
