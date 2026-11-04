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

/// Apply one of the thirteen map symbols without a mission-start prerequisite.
///
/// `StageMismatch` means the donor makes no mutation: a repeated symbol or a
/// completed collection whose mission already reached four. A collection with
/// ten objects but a lower mission still repairs that mission, without counting
/// another symbol. The durable caller must evaluate and commit all three tracks
/// together; this function does not read an advisory session copy.
pub fn apply_symbol(symbol: u8, snapshot: Snapshot) -> Result<Snapshot, QuestRefusal> {
    if symbol >= SYMBOL_COUNT {
        return Err(QuestRefusal::NotSupported);
    }
    let objects = snapshot.objects.max(0);
    if objects >= 10 {
        if snapshot.mission >= 4 {
            return Err(QuestRefusal::StageMismatch);
        }
        return Ok(Snapshot {
            mission: 4,
            ..snapshot
        });
    }

    let temp = snapshot.temp.max(0);
    let bit = 1_i64 << symbol;
    if temp & bit != 0 {
        return Err(QuestRefusal::StageMismatch);
    }
    let objects = objects + 1;
    Ok(Snapshot {
        mission: if objects == 10 { 4 } else { snapshot.mission },
        temp: temp | bit,
        objects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start() -> Snapshot {
        Snapshot {
            mission: 3,
            temp: 0,
            objects: 0,
        }
    }

    #[test]
    fn repeated_symbol_does_not_count_twice() -> Result<(), QuestRefusal> {
        let first = apply_symbol(12, start())?;
        assert_eq!(first.temp, 4096);
        assert_eq!(first.objects, 1);
        assert_eq!(apply_symbol(12, first), Err(QuestRefusal::StageMismatch));
        assert_eq!(apply_symbol(12, first), Err(QuestRefusal::StageMismatch));
        Ok(())
    }

    #[test]
    fn any_ten_distinct_symbols_complete_the_collection() -> Result<(), QuestRefusal> {
        let symbols = [12, 2, 9, 7, 1, 11, 0, 10, 4, 6];
        let mut snapshot = start();
        for (index, symbol) in symbols.into_iter().enumerate() {
            snapshot = apply_symbol(symbol, snapshot)?;
            assert_eq!(snapshot.objects, (index + 1) as i64);
            assert_eq!(snapshot.mission, if index == 9 { 4 } else { 3 });
        }
        assert_eq!(snapshot.temp.count_ones(), 10);
        assert_eq!(apply_symbol(3, snapshot), Err(QuestRefusal::StageMismatch));
        Ok(())
    }

    #[test]
    fn final_count_repairs_a_lower_mission_without_touching_count_or_bits() {
        let snapshot = Snapshot {
            mission: 2,
            temp: 1023,
            objects: 10,
        };
        assert_eq!(
            apply_symbol(12, snapshot),
            Ok(Snapshot {
                mission: 4,
                ..snapshot
            })
        );
    }

    #[test]
    fn complete_collection_keeps_later_mission_unchanged() {
        let snapshot = Snapshot {
            mission: 7,
            temp: 1023,
            objects: 10,
        };
        assert_eq!(apply_symbol(12, snapshot), Err(QuestRefusal::StageMismatch));
    }

    #[test]
    fn tenth_symbol_retains_donor_unconditional_mission_write() {
        let snapshot = Snapshot {
            mission: 7,
            temp: 511,
            objects: 9,
        };
        assert_eq!(
            apply_symbol(12, snapshot),
            Ok(Snapshot {
                mission: 4,
                temp: 4607,
                objects: 10,
            })
        );
    }

    #[test]
    fn no_mission_gate_and_negative_legacy_counters_normalize_as_donor() {
        assert_eq!(
            apply_symbol(
                0,
                Snapshot {
                    mission: -1,
                    temp: -1,
                    objects: -1,
                },
            ),
            Ok(Snapshot {
                mission: -1,
                temp: 1,
                objects: 1,
            })
        );
    }

    #[test]
    fn a_symbol_outside_the_pinned_table_is_refused() {
        assert_eq!(
            apply_symbol(SYMBOL_COUNT, start()),
            Err(QuestRefusal::NotSupported)
        );
    }
}
