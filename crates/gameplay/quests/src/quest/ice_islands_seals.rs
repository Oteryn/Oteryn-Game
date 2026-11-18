//! Ice Islands, Nibelor 3: three paintings of source-qualified map seals.
//!
//! Source: crystalserver@201189f871d23597b82caf88fd611454a2ddefb8,
//! data-global/scripts/quests/the_ice_islands_quest/actions_paint.lua:15-21.
//! TibiaWiki spoiler 17076/1138714 permits repainting after the map reset.
//! The caller owns tool custody and enforces the unpainted map appearance.
use super::QuestRefusal;

pub mod callbacks;
pub(crate) mod catalogue;
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
    if seal >= targets::TARGET_COUNT || snapshot.targets < 0 || snapshot.targets >> 48 != 0 {
        return Err(QuestRefusal::OutOfRange);
    }

    // Preserve the sorted, one-based slots in existing saves. Equal slots
    // record separate successful paintings of the same physical seal.
    let mut slots = [0_u16; 3];
    for (index, slot) in slots.iter_mut().enumerate() {
        *slot = ((snapshot.targets >> (index * 16)) & 0xffff) as u16;
    }
    let count = slots.iter().take_while(|&&slot| slot != 0).count();
    if slots[count..].iter().any(|&slot| slot != 0)
        || slots[..count].iter().any(|&slot| slot > targets::TARGET_COUNT)
        || slots[..count].windows(2).any(|pair| pair[0] > pair[1])
    {
        return Err(QuestRefusal::OutOfRange);
    }
    let slot = slots.get_mut(count).ok_or(QuestRefusal::StageMismatch)?;
    *slot = seal + 1;
    slots[..=count].sort_unstable();
    snapshot.targets = i64::from(slots[0])
        | (i64::from(slots[1]) << 16)
        | (i64::from(slots[2]) << 32);
    if count == 2 {
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
        assert!(paint(u16::MAX, base).is_err());
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
