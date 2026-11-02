//! The sixteen steps of Dance Dance Evolution, evaluated from one locked snapshot.
//!
//! Source: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/unnatural_selection/movements_mission3_dance_dance_evolution.lua:1-67.
//! The source guards only DanceStatus. Mission03 and Questline change only at
//! the final step; their earlier values do not grant or refuse a dance step.

use super::QuestRefusal;

pub const QUEST: &str = "oteryn:quest.unnatural_selection_quest";
pub const DANCE_STATUS_TRACK: &str =
    "oteryn:quest-progress/quest/u8_54/unnatural_selection/dance_status";
pub const MISSION03_TRACK: &str = "oteryn:quest-progress/quest/u8_54/unnatural_selection/mission03";
pub const QUESTLINE_TRACK: &str = "oteryn:quest-progress/quest/u8_54/unnatural_selection/questline";
pub const TRACKS: [&str; 3] = [DANCE_STATUS_TRACK, MISSION03_TRACK, QUESTLINE_TRACK];

/// Literal AID12333 tiles in row order, with native negative floors.
/// Tile identifiers are one-based indices into this list.
pub const TILES: [(u16, u16, i8); 9] = [
    (32990, 31496, -1),
    (32991, 31496, -1),
    (32992, 31496, -1),
    (32990, 31497, -1),
    (32991, 31497, -1),
    (32992, 31497, -1),
    (32990, 31498, -1),
    (32991, 31498, -1),
    (32992, 31498, -1),
];

/// Source config[1] through config[16], translated to the tile identifiers above.
pub const SEQUENCE: [u8; 16] = [5, 7, 5, 9, 5, 8, 4, 2, 6, 2, 5, 1, 5, 3, 5, 2];

/// Values of the three existing canonical tracks, read under the quest writer lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub dance_status: i64,
    pub mission03: i64,
    pub questline: i64,
}

/// Closed until the dedicated quest commit installs its source rule.
pub fn apply_dance(_tile: u8, _snapshot: Snapshot) -> Result<Snapshot, QuestRefusal> {
    Err(QuestRefusal::NotSupported)
}
