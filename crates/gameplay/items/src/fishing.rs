//! Global fishing rules shared by dispatch and the durable item owner.

pub const FISHING_ROD: &str = "oteryn:item.tibia.i3483";
pub const WORM: &str = "oteryn:item.tibia.i3492";
pub const FISH: &str = "oteryn:item.tibia.i3578";
pub const MECHANICAL_ROD: &str = "oteryn:item.tibia.i9306";
pub const NAIL: &str = "oteryn:item.tibia.i953";
pub const MECHANICAL_FISH: u16 = 9307;

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
    // Crystal data-crystal/scripts/actions/tools/skinning.lua:3-351,363,394.
    // Keep every simple corpse row in the donor table.
    let (spent_item, item, threshold) = match (tool, target) {
        (5908, 4272) => (4012, 5878, 7000),
        (5908, 5969) => (4012, 5878, 7000),
        (5908, 4052) => (4053, 5878, 7000),
        (5908, 5982) => (4053, 5878, 7000),
        (5908, 4047) => (4048, 5878, 7000),
        (5908, 5981) => (4048, 5878, 7000),
        (5908, 4057) => (4058, 5878, 7000),
        (5908, 5983) => (4058, 5878, 7000),
        (5908, 21092) => (21093, 5878, 7000),
        (5908, 21091) => (21093, 5878, 7000),
        (5908, 21096) => (21097, 5878, 7000),
        (5908, 21095) => (21097, 5878, 7000),
        (5908, 21100) => (21101, 5878, 7000),
        (5908, 21099) => (21101, 5878, 7000),
        (5908, 21000) => (21002, 5878, 7000),
        (5908, 21001) => (21002, 5878, 7000),
        (5908, 21004) => (21006, 5878, 7000),
        (5908, 21005) => (21006, 5878, 7000),
        (5908, 20996) => (20998, 5878, 7000),
        (5908, 20997) => (20998, 5878, 7000),
        (5908, 4324) => (4325, 5876, 6000),
        (5908, 6040) => (4325, 5876, 6000),
        (5908, 4327) => (4328, 5876, 6000),
        (5908, 6041) => (4328, 5876, 6000),
        (5908, 4321) => (4322, 5876, 6000),
        (5908, 4239) => (4322, 5876, 6000),
        (5908, 10368) => (10369, 5876, 10000),
        (5908, 10371) => (10369, 5876, 10000),
        (5908, 10360) => (10361, 5876, 10000),
        (5908, 10363) => (10361, 5876, 10000),
        (5908, 10352) => (10353, 5876, 10000),
        (5908, 10355) => (10353, 5876, 10000),
        (5908, 10364) => (10365, 5876, 10000),
        (5908, 10367) => (10365, 5876, 10000),
        (5908, 4286) => (4287, 5877, 5000),
        (5908, 5973) => (4287, 5877, 5000),
        (5908, 4062) => (4063, 5948, 5000),
        (5908, 5984) => (4063, 5948, 5000),
        (5908, 4112) => (4113, 5893, 10000),
        (5908, 5999) => (4113, 5893, 10000),
        (5908, 4212) => (4213, 5925, 6000),
        (5908, 6030) => (4213, 5925, 6000),
        (5908, 22743) => (22744, 24842, 50000),
        (5908, 22742) => (22744, 24842, 50000),
        (5908, 12172) => (4302, 6017, 6000),
        (5942, 4097) => (4098, 5906, 3000),
        (5942, 5995) => (4098, 5906, 3000),
        (5942, 4137) => (4138, 5905, 6000),
        (5942, 6006) => (4138, 5905, 6000),
        (5942, 8738) => (8742, 5905, 6000),
        (5942, 8744) => (8742, 5905, 6000),
        (5942, 18958) => (18959, 5905, 6000),
        (5942, 18961) => (18959, 5905, 6000),
        // Crystal data-global/scripts/actions/tools/skinning.lua:3-8 ICE_CONFIG: each stage
        // advances at 22344/100000 to the next sculpture; 7446 is final.
        (5908 | 5942, 7441 | 7442 | 7444 | 7445) => {
            let next = match target {
                7441 => 7442,
                7442 => 7444,
                7444 => 7445,
                _ => 7446,
            };
            return Some((
                GatheringTarget::Ice { spent_item: next },
                GatheringYield {
                    item: next,
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
        (9306, 7236) => {
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
        GatheringYield { item, threshold },
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

/// Crystal mechanical_fishing.lua:90-97,203: the Yalahar sewers box, native floor `-z` (z 8).
pub fn in_yalahar_sewers(x: u16, y: u16, floor: i8) -> bool {
    (32_736..=32_854).contains(&x) && (31_142..=31_260).contains(&y) && floor == -8
}

/// Crystal mechanical_fishing.lua:53-67,236-243: a fishable water the nail catch turns into its
/// non-fishable twin, which decays back to the fishable id after `duration` 1200 s
/// (items.xml:13998-14004, 14079-14085). `21312/21314` is outside the admitted `WATER` set.
pub fn nail_spent_water(target: u16) -> Option<u16> {
    match target {
        4597..=4602 => Some(target + 12),
        629..=634 => Some(target + 4_180),
        _ => None,
    }
}

pub const NAIL_WATER_DECAY_MS: i64 = 1_200_000;

/// The fishable id a spent nail-water decays back to (inverse of `nail_spent_water`).
pub fn nail_spent_water_decays_to(spent: u16) -> Option<u16> {
    match spent {
        4609..=4614 => Some(spent - 12),
        4809..=4814 => Some(spent - 4_180),
        _ => None,
    }
}

/// Rod admission for the Yalahar nail catch; the caller proves sewers position and a held nail.
pub fn accepts_nail_rod(tool_key: &str, water_key: &str) -> bool {
    tool_key == MECHANICAL_ROD
        && water_key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u16>().ok())
            .is_some_and(|id| nail_spent_water(id).is_some())
}

/// Crystal mechanical_fishing.lua:218-248: a skill-scaled catch spends one nail, turns the water
/// non-fishable and yields mechanical fish 9307. A miss changes nothing here.
pub fn nail_attempt(target: u16, skill: u16, catch_roll: u32) -> Option<GatheringAttempt> {
    let after = nail_spent_water(target)?;
    catches(skill, catch_roll).then_some(GatheringAttempt {
        after: Some(after),
        yield_item: Some(MECHANICAL_FISH),
        consumes_bait: true,
    })
}

/// The bait a gathering attempt may consume: a worm, or the nail for the mechanical fish.
pub fn is_gathering_bait(bait_key: &str, attempt: &GatheringAttempt) -> bool {
    bait_key == WORM || (bait_key == NAIL && attempt.yield_item == Some(MECHANICAL_FISH))
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

/// Rod admission used before dispatch trains fishing or consumes bait.
/// Crystal data-global/scripts/actions/other/mechanical_fishing.lua:334-346,378.
/// Only its ice-hole gathering slice is admitted for the mechanical rod here.
pub fn accepts_fishing_rod(tool_key: &str, water_key: &str) -> bool {
    (tool_key == FISHING_ROD && is_water(water_key))
        || (tool_key == "oteryn:item.tibia.i9306" && water_key == "oteryn:item.tibia.i7236")
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
    fn m09_yalahar_nail_catch_yields_mechanical_fish_and_spends_water() {
        // Crystal mechanical_fishing.lua:90-97,203-248; items.xml:13998-14004.
        assert!(in_yalahar_sewers(32_736, 31_142, -8));
        assert!(in_yalahar_sewers(32_854, 31_260, -8));
        assert!(!in_yalahar_sewers(32_735, 31_142, -8));
        assert!(!in_yalahar_sewers(32_800, 31_261, -8));
        assert!(!in_yalahar_sewers(32_800, 31_200, -7));
        assert!(accepts_nail_rod(MECHANICAL_ROD, "oteryn:item.tibia.i4597"));
        assert!(!accepts_nail_rod(FISHING_ROD, "oteryn:item.tibia.i4597"));
        assert!(!accepts_nail_rod(MECHANICAL_ROD, "oteryn:item.tibia.i4609"));
        assert!(!accepts_nail_rod(MECHANICAL_ROD, "oteryn:item.tibia.i7236"));
        for (water, spent) in [(4597, 4609), (4602, 4614), (629, 4809), (634, 4814)] {
            assert_eq!(nail_spent_water_decays_to(spent), Some(water));
            assert_eq!(
                nail_attempt(water, 10, 10_000),
                Some(GatheringAttempt {
                    after: Some(spent),
                    yield_item: Some(9307),
                    consumes_bait: true,
                })
            );
        }
        assert_eq!(nail_attempt(4597, 10, 10_001), None);
        assert_eq!(nail_attempt(4609, 10, 1), None);
        let nail = nail_attempt(4597, 10, 1);
        assert!(nail.is_some_and(|a| is_gathering_bait(NAIL, &a) && is_gathering_bait(WORM, &a)));
        let ice = gathering_attempt(9306, 7236, 10, 1, 1);
        assert!(ice.is_some_and(|a| is_gathering_bait(WORM, &a) && !is_gathering_bait(NAIL, &a)));
    }

    #[test]
    fn m09_ice_sculpture_chain_advances_at_22344_per_100000() {
        // Crystal data-global/scripts/actions/tools/skinning.lua:3-8 ICE_CONFIG.
        for (from, to) in [(7441, 7442), (7442, 7444), (7444, 7445), (7445, 7446)] {
            for tool in [5908, 5942] {
                assert_eq!(
                    gathering_attempt(tool, from, 10, 1, 22_344),
                    Some(GatheringAttempt {
                        after: Some(to),
                        yield_item: None,
                        consumes_bait: false,
                    }),
                    "{from}"
                );
                assert_eq!(
                    gathering_attempt(tool, from, 10, 1, 22_345),
                    Some(GatheringAttempt {
                        after: None,
                        yield_item: None,
                        consumes_bait: false,
                    }),
                    "{from} miss"
                );
            }
        }
        // 7446 is the last stage and cannot be carved further.
        assert!(gathering_rule(5908, 7446).is_none());
    }

    #[test]
    fn m09_mechanical_rod_is_admitted_on_ice_holes() {
        assert!(accepts_fishing_rod(
            "oteryn:item.tibia.i9306",
            "oteryn:item.tibia.i7236"
        ));
        assert!(!accepts_fishing_rod(
            "oteryn:item.tibia.i9306",
            "oteryn:item.tibia.i629"
        ));
        assert!(!accepts_fishing_rod(
            "oteryn:item.tibia.i3456",
            "oteryn:item.tibia.i7236"
        ));
        assert!(accepts_fishing_rod(FISHING_ROD, "oteryn:item.tibia.i629"));
    }

    #[test]
    fn m09_mechanical_rod_ice_hole_catch_and_loot_boundaries() {
        assert_eq!(gathering_attempt(9306, 7236, 10, 10_001, 1), None);
        assert_eq!(gathering_attempt(9306, 7236, 10, 1, 0), None);
        for (roll, item) in [
            (1, 7158),
            (1000, 7158),
            (1001, 3580),
            (4000, 3580),
            (4001, 7159),
            (10000, 7159),
            (10001, 3578),
            (100000, 3578),
        ] {
            assert_eq!(
                gathering_attempt(9306, 7236, 10, 10_000, roll),
                Some(GatheringAttempt {
                    after: Some(7237),
                    yield_item: Some(item),
                    consumes_bait: true,
                })
            );
        }
        assert!(gathering_rule(9306, 7237).is_none());
    }

    #[test]
    fn m09_all_corpse_rows_use_donor_thresholds_out_of_100000() {
        // Crystal data-crystal/scripts/actions/tools/skinning.lua:6-350,363,394.
        for (tool, target, spent, item, threshold) in [
            (5908, 4272, 4012, 5878, 7000),
            (5908, 5969, 4012, 5878, 7000),
            (5908, 4052, 4053, 5878, 7000),
            (5908, 5982, 4053, 5878, 7000),
            (5908, 4047, 4048, 5878, 7000),
            (5908, 5981, 4048, 5878, 7000),
            (5908, 4057, 4058, 5878, 7000),
            (5908, 5983, 4058, 5878, 7000),
            (5908, 21092, 21093, 5878, 7000),
            (5908, 21091, 21093, 5878, 7000),
            (5908, 21096, 21097, 5878, 7000),
            (5908, 21095, 21097, 5878, 7000),
            (5908, 21100, 21101, 5878, 7000),
            (5908, 21099, 21101, 5878, 7000),
            (5908, 21000, 21002, 5878, 7000),
            (5908, 21001, 21002, 5878, 7000),
            (5908, 21004, 21006, 5878, 7000),
            (5908, 21005, 21006, 5878, 7000),
            (5908, 20996, 20998, 5878, 7000),
            (5908, 20997, 20998, 5878, 7000),
            (5908, 4324, 4325, 5876, 6000),
            (5908, 6040, 4325, 5876, 6000),
            (5908, 4327, 4328, 5876, 6000),
            (5908, 6041, 4328, 5876, 6000),
            (5908, 4321, 4322, 5876, 6000),
            (5908, 4239, 4322, 5876, 6000),
            (5908, 10368, 10369, 5876, 10000),
            (5908, 10371, 10369, 5876, 10000),
            (5908, 10360, 10361, 5876, 10000),
            (5908, 10363, 10361, 5876, 10000),
            (5908, 10352, 10353, 5876, 10000),
            (5908, 10355, 10353, 5876, 10000),
            (5908, 10364, 10365, 5876, 10000),
            (5908, 10367, 10365, 5876, 10000),
            (5908, 4286, 4287, 5877, 5000),
            (5908, 5973, 4287, 5877, 5000),
            (5908, 4062, 4063, 5948, 5000),
            (5908, 5984, 4063, 5948, 5000),
            (5908, 4112, 4113, 5893, 10000),
            (5908, 5999, 4113, 5893, 10000),
            (5908, 4212, 4213, 5925, 6000),
            (5908, 6030, 4213, 5925, 6000),
            (5908, 22743, 22744, 24842, 50000),
            (5908, 22742, 22744, 24842, 50000),
            (5908, 12172, 4302, 6017, 6000),
            (5942, 4097, 4098, 5906, 3000),
            (5942, 5995, 4098, 5906, 3000),
            (5942, 4137, 4138, 5905, 6000),
            (5942, 6006, 4138, 5905, 6000),
            (5942, 8738, 8742, 5905, 6000),
            (5942, 8744, 8742, 5905, 6000),
            (5942, 18958, 18959, 5905, 6000),
            (5942, 18961, 18959, 5905, 6000),
        ] {
            assert_eq!(
                gathering_rule(tool, target),
                Some((
                    GatheringTarget::Corpse { spent_item: spent },
                    GatheringYield { item, threshold }
                ))
            );
            for roll in [1, threshold, threshold + 1, 100_000] {
                assert_eq!(
                    gathering_attempt(tool, target, 10, 1, roll),
                    Some(GatheringAttempt {
                        after: Some(spent),
                        yield_item: (roll <= threshold).then_some(item),
                        consumes_bait: false,
                    })
                );
            }
            for invalid in [0, 100_001] {
                assert_eq!(gathering_attempt(tool, target, 10, 1, invalid), None);
            }
        }
    }

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
            (5908, 5969, 4012, 5878, 7_000),
            (5908, 6040, 4325, 5876, 6_000),
            (5908, 5973, 4287, 5877, 5_000),
            (5908, 5984, 4063, 5948, 5_000),
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
                        threshold: if item == 5906 { 3_000 } else { 6_000 }
                    },
                ))
            );
            assert!(gathering_rule(3483, target).is_none());
        }
    }

    #[test]
    fn m09_corpse_attempt_marks_success_and_failure() {
        for (tool, target, spent, item, threshold) in [
            (5908, 5969, 4012, 5878, 7_000),
            (5942, 5995, 4098, 5906, 3_000),
        ] {
            assert_eq!(
                gathering_attempt(tool, target, 10, 100_000, threshold),
                Some(GatheringAttempt {
                    after: Some(spent),
                    yield_item: Some(item),
                    consumes_bait: false
                })
            );
            assert_eq!(
                gathering_attempt(tool, target, 10, 1, threshold + 1),
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
