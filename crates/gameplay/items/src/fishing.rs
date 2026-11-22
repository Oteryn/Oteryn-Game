//! Global fishing rules shared by dispatch and the durable item owner.

pub const FISHING_ROD: &str = "oteryn:item.tibia.i3483";
pub const WORM: &str = "oteryn:item.tibia.i3492";
pub const FISH: &str = "oteryn:item.tibia.i3578";
pub const NAIL: &str = "oteryn:item.tibia.i953";

pub const MECHANICAL_FISHING_WATER: [u16; 13] = [
    4597, 4598, 4599, 4600, 4601, 4602, 629, 630, 631, 632, 633, 634, 21312,
];

pub const MECHANICAL_FISHING_SPENT_WATER: [u16; 13] = [
    4609, 4610, 4611, 4612, 4613, 4614, 4809, 4810, 4811, 4812, 4813, 4814, 21314,
];

/// Result of the Yalahar-sewer branch in Crystal's mechanical rod action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MechanicalSewerAttempt {
    NotHandled,
    SpentWater,
    Miss,
    Shocked,
    Caught { after: u16 },
}

/// Crystal `data-global/scripts/actions/other/mechanical_fishing.lua:203-248`.
/// It classifies every donor branch; the durable gathering caller uses `Caught` to consume
/// one nail, transform the water and mint the mechanical fish atomically.
pub fn mechanical_sewer_attempt(
    target: u16,
    in_sewers: bool,
    has_nail: bool,
    caught: bool,
    shock_roll: u32,
) -> MechanicalSewerAttempt {
    if !in_sewers || !has_nail {
        return MechanicalSewerAttempt::NotHandled;
    }
    if MECHANICAL_FISHING_SPENT_WATER.contains(&target) {
        return MechanicalSewerAttempt::SpentWater;
    }
    if !MECHANICAL_FISHING_WATER.contains(&target) {
        return MechanicalSewerAttempt::NotHandled;
    }
    if !caught {
        return if (1..=10).contains(&shock_roll) {
            MechanicalSewerAttempt::Shocked
        } else {
            MechanicalSewerAttempt::Miss
        };
    }
    let Some(after) = spent_water(target) else {
        return MechanicalSewerAttempt::NotHandled;
    };
    MechanicalSewerAttempt::Caught { after }
}

fn spent_water(target: u16) -> Option<u16> {
    Some(match target {
        4597..=4602 => target + 12,
        629..=634 => target + 4_180,
        21312 => 21314,
        _ => return None,
    })
}

/// Crystal mechanical_fishing.lua:85-97,203-248.
pub fn is_yalahar_sewer_water(target: u16, x: u16, y: u16, floor: i8) -> bool {
    MECHANICAL_FISHING_WATER.contains(&target) && in_yalahar_sewers(x, y, floor)
}

pub fn in_yalahar_sewers(x: u16, y: u16, floor: i8) -> bool {
    (32736..=32854).contains(&x) && (31142..=31260).contains(&y) && floor == -8
}

/// Bait and outcome are selected by the rod, the exact tile and Yalahar's sewer condition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaterFishing {
    Ordinary,
    MechanicalSewer,
    MechanicalWater,
    Ice,
    Dirty,
    DirtyLoot,
    Desert,
    MechanicalDesert,
    Spent,
}

pub fn water_fishing(
    tool: u16,
    target: u16,
    in_sewers: bool,
    has_nail: bool,
) -> Option<WaterFishing> {
    use WaterFishing::*;
    Some(match (tool, target) {
        (3483 | 9306, 7236) => Ice,
        (3483, 13988) => Desert,
        (9306, 13989) => MechanicalDesert,
        (3483, 12560) => DirtyLoot,
        (3483, 13989) | (9306, 13988 | 622 | 12558..=12560) => Spent,
        (3483 | 9306, 4609..=4614 | 4809..=4814 | 21314) => Spent,
        (9306, target) if in_sewers && has_nail && MECHANICAL_FISHING_WATER.contains(&target) => {
            MechanicalSewer
        }
        (9306, 12561..=12563) => Dirty,
        (9306, 629..=634 | 4597..=4602 | 21312) => MechanicalWater,
        (3483, 629..=634 | 4597..=4602 | 12561..=12563 | 21414) => Ordinary,
        _ => return None,
    })
}

impl WaterFishing {
    pub fn bait(self) -> &'static str {
        if self == Self::MechanicalSewer {
            NAIL
        } else {
            WORM
        }
    }

    /// Called only after a skill-scaled catch. These buckets are out of 100,000.
    pub fn special_catch(self, target: u16, loot_roll: u32) -> Option<GatheringAttempt> {
        use WaterFishing::*;
        let (after, yield_item) = match self {
            Ordinary | Spent => return None,
            // CRAFT-1 C4b records the nested donor draw as weights out of 600.
            DirtyLoot => (12560, Some(dirty_water_loot(loot_roll)?)),
            Ice => (
                7237,
                Some(match loot_roll {
                    1..=1_000 => 7158,
                    1_001..=4_000 => 3580,
                    4_001..=10_000 => 7159,
                    _ => 3578,
                }),
            ),
            MechanicalSewer => (spent_water(target)?, Some(9307)),
            MechanicalWater => (spent_water(target)?, Some(3578)),
            // Crystal fishing.lua:57-68. The upper four integers of math.random(100).
            Desert => (13989, Some(if loot_roll > 96_000 { 13992 } else { 3578 })),
            // Crystal mechanical_fishing.lua:170-195. The spent form has a 20-minute decay.
            MechanicalDesert => (13988, (loot_roll <= 1_000).then_some(13992)),
            // Crystal mechanical_fishing.lua:108-166. A 30% loot branch, with a 1% rare
            // draw inside it, is 300 swimmer and 29,700 trash outcomes per 100,000.
            Dirty => (
                target.checked_sub(3)?,
                match loot_roll {
                    1..=300 => Some(12557),
                    301..=30_000 => Some(3111),
                    _ => None,
                },
            ),
        };
        Some(GatheringAttempt {
            after: Some(after),
            yield_item,
            consumes_bait: self != DirtyLoot,
        })
    }

    pub fn decay(self, target: u16) -> Vec<(u32, i64)> {
        match self {
            Self::Ice => vec![(7200, 900_000)],
            Self::MechanicalSewer
            | Self::MechanicalWater
            | Self::Dirty
            | Self::MechanicalDesert => vec![(u32::from(target), 1_200_000)],
            _ => Vec::new(),
        }
    }
}

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
        // Pinned Crystal skinning.lua:24–26. The donor marks this chance unconfirmed.
        (5908, 52678..=52680) => (52677, 52632, 10_000),
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
        (5908, 7441) | (5942, 7441) => {
            return Some((
                GatheringTarget::Ice { spent_item: 7442 },
                GatheringYield {
                    item: 7442,
                    threshold: 22_000,
                },
            ));
        }
        (5908, 10426) => {
            return Some((
                GatheringTarget::Corpse { spent_item: 0 },
                GatheringYield {
                    item: 10429,
                    threshold: 24_000,
                },
            ));
        }
        (5908, 7442) => {
            return Some((
                GatheringTarget::Ice { spent_item: 7444 },
                GatheringYield {
                    item: 7444,
                    threshold: 4_800,
                },
            ));
        }
        (5908, 7444) => {
            return Some((
                GatheringTarget::Ice { spent_item: 7445 },
                GatheringYield {
                    item: 7445,
                    threshold: 900,
                },
            ));
        }
        (5908, 7445) => {
            return Some((
                GatheringTarget::Ice { spent_item: 7446 },
                GatheringYield {
                    item: 7446,
                    threshold: 40,
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
        (9306, target) if MECHANICAL_FISHING_WATER.contains(&target) => {
            return Some((
                GatheringTarget::FishingWater { spent_item: target },
                GatheringYield {
                    item: 9307,
                    threshold: 100_000,
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
    if tool == 9306 && MECHANICAL_FISHING_WATER.contains(&target) {
        if !(1..=100_000).contains(&yield_roll) || !catches(skill, catch_roll) {
            return None;
        }
        let MechanicalSewerAttempt::Caught { after } =
            mechanical_sewer_attempt(target, true, true, true, 100)
        else {
            return None;
        };
        return Some(GatheringAttempt {
            after: Some(after),
            yield_item: Some(9307),
            consumes_bait: true,
        });
    }
    if tool == 5908 && target == 10426 {
        if !(1..=100_000).contains(&yield_roll) {
            return None;
        }
        return Some(GatheringAttempt {
            after: None,
            yield_item: match yield_roll {
                1..=530 => Some(10429),
                531..=9_600 => Some(10428),
                9_601..=24_000 => Some(10427),
                _ => None,
            },
            consumes_bait: false,
        });
    }
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
            debug_assert_eq!(spent_item, 7237);
            WaterFishing::Ice.special_catch(target, yield_roll)?
        }
    })
}

pub fn is_fishing_corpse(tool: u16, target: u16) -> bool {
    matches!((tool, target), (3483 | 9306, 9582))
}

/// Crystal data-crystal/scripts/actions/tools/fishing.lua:3-7,17-38 and
/// data-global/scripts/actions/other/mechanical_fishing.lua:62-71,272-295.
pub fn corpse_fishing_yield(tool: u16, draw: u64) -> Option<u16> {
    match tool {
        3483 => {
            // The donor first chooses a rarity, then an equiprobable item from that tier.
            // Each tier is represented exactly on a common 6,000-outcome denominator.
            let roll = usize::try_from(draw % 6000).ok()?;
            Some(match roll {
                0..=59 => [281, 282, 9303, 12557][roll / 15],
                60..=179 => [3026, 3029, 3032, 7158, 7159][(roll - 60) / 24],
                180..=599 => [3035, 3051, 3052, 3580, 236, 237][(roll - 180) / 70],
                _ => [3119, 3123, 3264, 3409, 3578][(roll - 600) / 1080],
            })
        }
        9306 => match draw % 10_000 + 1 {
            1..=500 => Some(3026),
            501..=801 => Some(3029),
            802..=1002 => Some(3032),
            1003..=1053 => Some(281),
            1054..=1104 => Some(282),
            1105..=1115 => Some(9303),
            _ => None,
        },
        _ => None,
    }
}

/// CRAFT-1 C4b, exact nested 1/2/7/90 draw for a dirty-water conversion.
fn dirty_water_loot(roll: u32) -> Option<u16> {
    Some(match roll {
        1..=3 => 281,
        4..=26 => 12557,
        27..=32 => 3026,
        33..=46 => 3035,
        47..=60 => 237,
        61..=168 => 3119,
        169..=276 => 3123,
        277..=384 => 3264,
        385..=492 => 3409,
        493..=600 => 3578,
        _ => return None,
    })
}

pub const WATER: [u16; 27] = [
    629, 630, 631, 632, 633, 634, 4597, 4598, 4599, 4600, 4601, 4602, 4609, 4610, 4611, 4612, 4613,
    4614, 7236, 9582, 12560, 12562, 12561, 12563, 13988, 13989, 21414,
];

/// Global's chance, represented in thousandths of one percent to avoid floats.
pub fn catch_threshold(skill: u16) -> u32 {
    let scaled = 10_000_i64 + (i64::from(skill) - 10) * 597;
    scaled.clamp(10_000, 50_000) as u32
}

pub fn catches(skill: u16, roll: u32) -> bool {
    // Each group of 1,000 equiprobable draws maps to one integer percent (CRAFT-1 C4).
    (1..=100_000).contains(&roll) && roll.div_ceil(1_000) * 1_000 <= catch_threshold(skill)
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
    fn catches_uses_the_craft1_integer_percent_draw() {
        // CRAFT-1 C4: uniform U in 1..=100; 1,000 * U <= the scaled curve.
        for (skill, expected) in [(0, 10_000), (20, 15_000), (77, 49_000), (78, 50_000)] {
            assert_eq!(
                (1..=100_000).filter(|&roll| catches(skill, roll)).count(),
                expected
            );
        }
        assert!(catches(20, 15_000));
        assert!(!catches(20, 15_001));
        assert!(!catches(20, 15_970));
        assert!(!catches(20, 0));
        assert!(!catches(20, 100_001));
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
    fn m09_mechanical_fishing_sewer_bounds_and_fish_are_pinned() {
        // Crystal data-global/scripts/actions/other/mechanical_fishing.lua:85-97,203-248.
        assert!(is_yalahar_sewer_water(4597, 32736, 31142, -8));
        assert!(is_yalahar_sewer_water(21312, 32854, 31260, -8));
        assert!(!is_yalahar_sewer_water(4597, 32735, 31142, -8));
        assert!(!is_yalahar_sewer_water(4597, 32736, 31142, -7));
        assert_eq!(
            gathering_attempt(9306, 4597, 10, 10_000, 1),
            Some(GatheringAttempt {
                after: Some(4609),
                yield_item: Some(9307),
                consumes_bait: true,
            })
        );
        assert_eq!(gathering_attempt(9306, 4597, 10, 10_001, 1), None);
    }

    #[test]
    fn m09_mechanical_fishing_covers_every_sewer_branch() {
        // Crystal data-global/scripts/actions/other/mechanical_fishing.lua:203-248.
        assert_eq!(
            mechanical_sewer_attempt(4597, false, true, true, 1),
            MechanicalSewerAttempt::NotHandled
        );
        assert_eq!(
            mechanical_sewer_attempt(4597, true, false, true, 1),
            MechanicalSewerAttempt::NotHandled
        );
        assert_eq!(
            mechanical_sewer_attempt(4609, true, true, true, 1),
            MechanicalSewerAttempt::SpentWater
        );
        assert_eq!(
            mechanical_sewer_attempt(4597, true, true, false, 11),
            MechanicalSewerAttempt::Miss
        );
        assert_eq!(
            mechanical_sewer_attempt(4597, true, true, false, 10),
            MechanicalSewerAttempt::Shocked
        );
        for (target, after) in [(4597, 4609), (629, 4809), (21312, 21314)] {
            assert_eq!(
                mechanical_sewer_attempt(target, true, true, true, 100),
                MechanicalSewerAttempt::Caught { after }
            );
        }
    }

    #[test]
    fn m09_both_sculpting_tools_reach_the_non_simple_ice_row() {
        // Crystal data-crystal/scripts/actions/tools/skinning.lua:289-305,392-403.
        for tool in [5908, 5942] {
            assert!(gathering_rule(tool, 7441).is_some());
            assert_eq!(
                gathering_attempt(tool, 7441, 10, 1, 22_000),
                Some(GatheringAttempt {
                    after: Some(7442),
                    yield_item: None,
                    consumes_bait: false,
                })
            );
            assert_eq!(
                gathering_attempt(tool, 7441, 10, 1, 22_001),
                Some(GatheringAttempt {
                    after: None,
                    yield_item: None,
                    consumes_bait: false,
                })
            );
        }
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
    fn m09_marble_sculpture_uses_ordered_chance_tiers_and_removes_the_rock() {
        // Crystal data-crystal/scripts/actions/tools/skinning.lua:266-288,363-391.
        for (roll, item) in [
            (1, Some(10429)),
            (530, Some(10429)),
            (531, Some(10428)),
            (9_600, Some(10428)),
            (9_601, Some(10427)),
            (24_000, Some(10427)),
            (24_001, None),
            (100_000, None),
        ] {
            assert_eq!(
                gathering_attempt(5908, 10426, 10, 1, roll),
                Some(GatheringAttempt {
                    after: None,
                    yield_item: item,
                    consumes_bait: false,
                })
            );
        }
    }

    #[test]
    fn m09_ice_sculpture_chain_uses_each_donor_threshold() {
        // Crystal data-crystal/scripts/actions/tools/skinning.lua:289-305,392-403.
        for (target, next, threshold) in [
            (7441, 7442, 22_000),
            (7442, 7444, 4_800),
            (7444, 7445, 900),
            (7445, 7446, 40),
        ] {
            assert_eq!(
                gathering_attempt(5908, target, 10, 1, threshold),
                Some(GatheringAttempt {
                    after: Some(next),
                    yield_item: None,
                    consumes_bait: false,
                })
            );
            assert_eq!(
                gathering_attempt(5908, target, 10, 1, threshold + 1),
                Some(GatheringAttempt {
                    after: None,
                    yield_item: None,
                    consumes_bait: false,
                })
            );
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
            // Crystal skinning.lua:290-293 pins 22,000 out of 100,000.
            threshold: 22_000,
        };
        assert_eq!(
            gathering_rule(5908, 7441),
            Some((GatheringTarget::Ice { spent_item: 7442 }, ice_row))
        );
        assert_eq!(gathering_yield(ice_row, 22_001), None);
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
                gathering_attempt(tool, 7441, 10, 1, 22_000),
                Some(GatheringAttempt {
                    after: Some(7442),
                    yield_item: None,
                    consumes_bait: false
                })
            );
            assert_eq!(
                gathering_attempt(tool, 7441, 10, 1, 22_001),
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

#[cfg(test)]
mod m41_tests;
