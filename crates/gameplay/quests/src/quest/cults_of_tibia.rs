//! Closed Barkless symbol rule, evaluated from one locked quest snapshot.
//!
//! Source: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/cults_of_tibia/actions_cult_symbol.lua:1-52,
//! data/libs/functions/bit.lua:10-41 (membership test and bitwise OR).
//! The canonical tracks are already owned by this quest in quest-state.json.

use super::QuestRefusal;

pub const QUEST: &str = "oteryn:quest.cults_of_tibia_quest";
pub const MISSION: &str = "oteryn:quest-progress/quest/u11_40/cults_of_tibia/barkless/mission";
pub const TEMP: &str = "oteryn:quest-progress/quest/u11_40/cults_of_tibia/barkless/temp";
pub const OBJECTS: &str = "oteryn:quest-progress/quest/u11_40/cults_of_tibia/barkless/objects";
pub const SYMBOL_COUNT: u8 = 13;

/// Source symbol order, with native negative floors. The Use selector also
/// requires the source action binding 5535 on the selected world item.
pub const SYMBOL_POSITIONS: [(u16, u16, i8); 13] = [
    (32731, 31531, -9),
    (32739, 31489, -9),
    (32739, 31507, -9),
    (32761, 31518, -9),
    (32720, 31545, -8),
    (32745, 31423, -8),
    (32742, 31410, -8),
    (32685, 31430, -8),
    (32746, 31462, -8),
    (32683, 31537, -9),
    (32740, 31494, -9),
    (32741, 31494, -9),
    (32745, 31523, -9),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub mission: i64,
    pub temp: i64,
    pub objects: i64,
}

/// Closed until the dedicated quest commit installs its source rule.
pub fn apply_symbol(_symbol: u8, _snapshot: Snapshot) -> Result<Snapshot, QuestRefusal> {
    Err(QuestRefusal::NotSupported)
}
