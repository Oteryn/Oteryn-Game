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

/// Pure planning inside the existing fenced quest transaction. No inventory,
/// achievement or presentation writes are hidden in this state transition.
pub fn apply_dance(tile: u8, mut snapshot: Snapshot) -> Result<Snapshot, QuestRefusal> {
    if tile == 0 || usize::from(tile) > TILES.len() {
        return Err(QuestRefusal::NotSupported);
    }
    let expected = snapshot
        .dance_status
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| SEQUENCE.get(index))
        .ok_or(QuestRefusal::StageMismatch)?;
    if *expected != tile {
        snapshot.dance_status = 1;
        return Ok(snapshot);
    }
    snapshot.dance_status += 1;
    if snapshot.dance_status == 17 {
        snapshot.mission03 = 3;
        snapshot.questline = 7;
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dance_uses_every_source_tile_and_all_sixteen_steps() -> Result<(), QuestRefusal> {
        let mut state = Snapshot {
            dance_status: 1,
            mission03: 2,
            questline: 6,
        };
        for (index, tile) in SEQUENCE.into_iter().enumerate() {
            state = apply_dance(tile, state)?;
            assert_eq!(
                state.dance_status,
                i64::try_from(index).map_err(|_| QuestRefusal::OutOfRange)? + 2
            );
            if index < 15 {
                assert_eq!((state.mission03, state.questline), (2, 6));
            }
        }
        assert_eq!(
            state,
            Snapshot {
                dance_status: 17,
                mission03: 3,
                questline: 7
            }
        );
        Ok(())
    }

    #[test]
    fn every_wrong_active_tile_resets_only_dance_status() -> Result<(), QuestRefusal> {
        for (index, expected) in SEQUENCE.into_iter().enumerate() {
            for tile in 1..=9 {
                if tile == expected {
                    continue;
                }
                let state = Snapshot {
                    dance_status: i64::try_from(index).map_err(|_| QuestRefusal::OutOfRange)? + 1,
                    mission03: 1,
                    questline: 12,
                };
                assert_eq!(
                    apply_dance(tile, state)?,
                    Snapshot {
                        dance_status: 1,
                        ..state
                    }
                );
            }
        }
        Ok(())
    }

    #[test]
    fn inactive_or_finished_dance_has_no_transition() {
        for dance_status in [-1, 0, 17, i64::MAX] {
            let state = Snapshot {
                dance_status,
                mission03: 2,
                questline: 6,
            };
            assert_eq!(apply_dance(5, state), Err(QuestRefusal::StageMismatch));
        }
    }

    #[test]
    fn final_step_preserves_the_sources_absence_of_mission_guards() {
        let state = Snapshot {
            dance_status: 16,
            mission03: 0,
            questline: 15,
        };
        assert_eq!(
            apply_dance(2, state),
            Ok(Snapshot {
                dance_status: 17,
                mission03: 3,
                questline: 7
            })
        );
    }

    #[test]
    fn unknown_tile_never_changes_a_track() {
        let state = Snapshot {
            dance_status: 1,
            mission03: 2,
            questline: 6,
        };
        for tile in [0, 10, u8::MAX] {
            assert_eq!(apply_dance(tile, state), Err(QuestRefusal::NotSupported));
        }
    }
}
