//! Global fishing rules shared by dispatch and the durable item owner.

pub const FISHING_ROD: &str = "oteryn:item.tibia.i3483";
pub const WORM: &str = "oteryn:item.tibia.i3492";
pub const FISH: &str = "oteryn:item.tibia.i3578";

/// The donor target kind determines when a target is spent and where its yield belongs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GatheringTarget {
    Corpse { spent_item: u16 },
    FishingWater { spent_item: u16 },
    Ice { spent_item: u16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GatheringYield {
    pub item: u16,
    pub threshold: u32,
}

/// Global/Crystal-compatible, bounded gathering rows used by the item-use dispatcher.
/// Thresholds are out of 100,000, as used by the donor skinning script.
pub fn gathering_rule(tool: u16, target: u16) -> Option<(GatheringTarget, GatheringYield)> {
    // Crystal skinning.lua:1,29,51,71,75,109,113; mutations:333-339; tools:356.
    let (spent_item, item) = match (tool, target) {
        (5908, 5969) => (4012, 5878),
        (5908, 6040) => (4325, 5876),
        (5908, 5973) => (4026, 5877),
        (5908, 5984) => (4063, 5948),
        (5942, 5995) => (4098, 5906),
        (5942, 6006) => (4138, 5905),
        (5908 | 5942, 7441) => {
            return Some((
                GatheringTarget::Ice { spent_item: 7442 },
                GatheringYield {
                    item: 7442,
                    threshold: 22_344,
                },
            ));
        }
        (3483, 7236) => {
            return Some((
                GatheringTarget::FishingWater { spent_item: 7237 },
                GatheringYield {
                    item: 7158,
                    threshold: 1_000,
                },
            ));
        }
        _ => return None,
    };
    Some((
        GatheringTarget::Corpse { spent_item },
        GatheringYield {
            item,
            threshold: 25_000,
        },
    ))
}

pub fn gathering_yield(row: GatheringYield, roll: u32) -> Option<u16> {
    ((1..=100_000).contains(&roll) && roll <= row.threshold).then_some(row.item)
}

/// A successful sculpture stays on the map; only skins/dust/fish enter inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GatheringAttempt {
    pub after: Option<u16>,
    pub yield_item: Option<u16>,
    pub consumes_bait: bool,
}

/// Crystal skinning.lua:158-172,310-339; fishing.lua:52-81.
/// Failed catches leave the hole untouched; corpse attempts always mark the corpse.
pub fn gathering_attempt(
    tool: u16,
    target: u16,
    skill: u16,
    catch_roll: u32,
    yield_roll: u32,
) -> Option<GatheringAttempt> {
    let (kind, row) = gathering_rule(tool, target)?;
    if !(1..=100_000).contains(&yield_roll) {
        return None;
    }
    Some(match kind {
        GatheringTarget::Corpse { spent_item } => GatheringAttempt {
            after: Some(spent_item),
            yield_item: gathering_yield(row, yield_roll),
            consumes_bait: false,
        },
        GatheringTarget::Ice { spent_item } => GatheringAttempt {
            after: (yield_roll <= row.threshold).then_some(spent_item),
            yield_item: None,
            consumes_bait: false,
        },
        GatheringTarget::FishingWater { spent_item } => {
            if !catches(skill, catch_roll) {
                return None;
            }
            GatheringAttempt {
                after: Some(spent_item),
                yield_item: Some(match yield_roll {
                    1..=1_000 => 7158,
                    1_001..=4_000 => 3580,
                    4_001..=10_000 => 7159,
                    _ => 3578,
                }),
                consumes_bait: true,
            }
        }
    })
}

pub const WATER: [u16; 26] = [
    629, 630, 631, 632, 633, 634, 4597, 4598, 4599, 4600, 4601, 4602, 4609, 4610, 4611, 4612, 4613,
    4614, 7236, 9582, 12560, 12562, 12561, 12563, 13988, 13989,
];

/// Global's chance, represented in thousandths of one percent to avoid floats.
pub fn catch_threshold(skill: u16) -> u32 {
    let scaled = 10_000_i64 + (i64::from(skill) - 10) * 597;
    scaled.clamp(10_000, 50_000) as u32
}

pub fn catches(skill: u16, roll: u32) -> bool {
    (1..=100_000).contains(&roll) && roll <= catch_threshold(skill)
}

pub fn is_water(production_key: &str) -> bool {
    production_key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse::<u16>().ok())
        .is_some_and(|id| WATER.contains(&id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_curve_is_clamped_and_uses_the_reference_slope() {
        assert_eq!(catch_threshold(10), 10_000);
        assert_eq!(catch_threshold(20), 15_970);
        assert_eq!(catch_threshold(77), 49_999);
        assert_eq!(catch_threshold(78), 50_000);
        assert!(catches(10, 10_000));
        assert!(!catches(10, 10_001));
    }

    #[test]
    fn only_the_global_water_catalogue_is_accepted() {
        assert!(is_water("oteryn:item.tibia.i4597"));
        assert!(is_water("oteryn:item.tibia.i13989"));
        assert!(!is_water("oteryn:item.tibia.i4596"));
    }

    #[test]
    fn seeded_gathering_rows_pin_yields_and_failed_attempts_spend_targets() {
        for (tool, target, spent, item, threshold) in [
            (5908, 5969, 4012, 5878, 25_000),
            (5908, 6040, 4325, 5876, 25_000),
            (5908, 5973, 4026, 5877, 25_000),
            (5908, 5984, 4063, 5948, 25_000),
        ] {
            let row = GatheringYield { item, threshold };
            assert_eq!(
                gathering_rule(tool, target),
                Some((GatheringTarget::Corpse { spent_item: spent }, row))
            );
            assert_eq!(gathering_yield(row, threshold), Some(item));
            assert_eq!(gathering_yield(row, threshold + 1), None);
        }
        let ice_row = GatheringYield {
            item: 7442,
            threshold: 22_344,
        };
        assert_eq!(
            gathering_rule(5908, 7441),
            Some((GatheringTarget::Ice { spent_item: 7442 }, ice_row))
        );
        assert_eq!(gathering_yield(ice_row, 22_345), None);
        let fishing_row = GatheringYield {
            item: 7158,
            threshold: 1_000,
        };
        assert_eq!(
            gathering_rule(3483, 7236),
            Some((
                GatheringTarget::FishingWater { spent_item: 7237 },
                fishing_row
            ))
        );
        assert_eq!(gathering_yield(fishing_row, 1_000), Some(7158));
    }
    #[test]
    fn m09_dusting_uses_the_blessed_wooden_stake() {
        for (target, spent, item) in [(5995, 4098, 5906), (6006, 4138, 5905)] {
            assert_eq!(
                gathering_rule(5942, target),
                Some((
                    GatheringTarget::Corpse { spent_item: spent },
                    GatheringYield {
                        item,
                        threshold: 25_000
                    },
                ))
            );
            assert!(gathering_rule(3483, target).is_none());
        }
    }

    #[test]
    fn m09_corpse_attempt_marks_success_and_failure() {
        for (tool, target, spent, item) in [(5908, 5969, 4012, 5878), (5942, 5995, 4098, 5906)] {
            assert_eq!(
                gathering_attempt(tool, target, 10, 100_000, 25_000),
                Some(GatheringAttempt {
                    after: Some(spent),
                    yield_item: Some(item),
                    consumes_bait: false
                })
            );
            assert_eq!(
                gathering_attempt(tool, target, 10, 1, 25_001),
                Some(GatheringAttempt {
                    after: Some(spent),
                    yield_item: None,
                    consumes_bait: false
                })
            );
        }
    }
    #[test]
    fn m09_ice_success_is_map_only_and_failure_removes_block() {
        for tool in [5908, 5942] {
            assert_eq!(
                gathering_attempt(tool, 7441, 10, 1, 22_344),
                Some(GatheringAttempt {
                    after: Some(7442),
                    yield_item: None,
                    consumes_bait: false
                })
            );
            assert_eq!(
                gathering_attempt(tool, 7441, 10, 1, 22_345),
                Some(GatheringAttempt {
                    after: None,
                    yield_item: None,
                    consumes_bait: false
                })
            );
        }
    }
    #[test]
    fn m09_ice_hole_changes_only_after_catch_and_rare_is_one_percent() {
        assert_eq!(gathering_attempt(3483, 7236, 10, 10_001, 1), None);
        for (roll, item) in [
            (1, 7158),
            (1000, 7158),
            (1001, 3580),
            (4000, 3580),
            (4001, 7159),
            (10_000, 7159),
            (10_001, 3578),
            (100_000, 3578),
        ] {
            assert_eq!(
                gathering_attempt(3483, 7236, 10, 10_000, roll),
                Some(GatheringAttempt {
                    after: Some(7237),
                    yield_item: Some(item),
                    consumes_bait: true
                })
            );
        }
    }
}
