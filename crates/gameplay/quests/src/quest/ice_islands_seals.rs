//! Ice Islands, Nibelor 3: three distinct source-qualified map seals.
//!
//! Source: crystalserver@201189f871d23597b82caf88fd611454a2ddefb8,
//! data-global/scripts/quests/the_ice_islands_quest/actions_paint.lua:15-21.
//! The chosen quest recipe additionally requires distinct targets. This planner
//! changes progress only; its caller owns tool custody and map-object authority.
use super::{QuestRefusal, distinct_targets};

pub mod callbacks;
mod custody;
pub use custody::consumes_paint;
pub mod targets;

pub const PAINT: &str = "oteryn:item.tibia.i7253";
pub const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
pub const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline";
pub const TARGETS: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/painted_seal_targets";
pub const MISSION04: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission04";
pub const TRACKS: [&str; 3] = [QUESTLINE, TARGETS, MISSION04];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub questline: i64,
    pub targets: i64,
    pub mission04: i64,
}

pub fn paint(seal: u16, mut snapshot: Snapshot) -> Result<Snapshot, QuestRefusal> {
    if snapshot.questline != 8 {
        return Err(QuestRefusal::StageMismatch);
    }
    let (targets, complete) =
        distinct_targets::record_three(snapshot.targets, seal, targets::TARGET_COUNT)?;
    snapshot.targets = targets;
    if complete {
        snapshot.questline = 9;
        snapshot.mission04 = 2;
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_distinct_seals_advance_only_the_final_snapshot() -> Result<(), QuestRefusal> {
        let mut state = Snapshot {
            questline: 8,
            targets: 0,
            mission04: 1,
        };
        for (index, seal) in [183, 2, 63].into_iter().enumerate() {
            state = paint(seal, state)?;
            assert_eq!(
                (state.questline, state.mission04),
                if index == 2 { (9, 2) } else { (8, 1) }
            );
            assert!(paint(seal, state).is_err());
        }
        assert_eq!(state.targets, 3 | (64 << 16) | (184 << 32));
        assert_eq!(paint(1, state), Err(QuestRefusal::StageMismatch));
        Ok(())
    }

    #[test]
    fn wrong_phase_corrupt_progress_and_unknown_targets_are_refused() {
        let base = Snapshot {
            questline: 8,
            targets: 0,
            mission04: -1,
        };
        for questline in [-1, 0, 7, 9, i64::MAX] {
            assert_eq!(
                paint(0, Snapshot { questline, ..base }),
                Err(QuestRefusal::StageMismatch)
            );
        }
        assert!(paint(distinct_targets::MAX_TARGETS, base).is_err());
        assert!(paint(targets::TARGET_COUNT, base).is_err());
        assert!(
            paint(
                0,
                Snapshot {
                    targets: -1,
                    ..base
                }
            )
            .is_err()
        );
    }
}
