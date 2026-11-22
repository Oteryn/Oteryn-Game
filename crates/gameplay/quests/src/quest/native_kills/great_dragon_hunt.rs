//! Crystal 201189f8: dragons/dragon.lua:17, dragons/dragon_lord.lua:17 and
//! the_great_dragon_hunt_quest/creaturescripts_the_great_dragon_hunt.lua:1-27.
use super::Counter;

const QUEST: &str = "oteryn:quest.adventurers_guild";
const TRACK: &str = "oteryn:quest-progress/quest/u10_80/the_great_dragon_hunt/dragon_counter";
const SKELETON: &str = "oteryn:quest-progress/quest/u10_80/the_great_dragon_hunt/warrior_skeleton";

const fn counter(transition: &'static str, creature: &'static str) -> Counter {
    Counter {
        transition,
        creature,
        quest: QUEST,
        track: TRACK,
        gate: SKELETON,
        maximum: i32::MAX as i64,
        authored_stage: false,
        world_change: None,
    }
}

pub(super) const COUNTERS: &[Counter] = &[
    counter(
        "oteryn:quest-transition/native-kills/great-dragon-hunt/dragon",
        "oteryn:creature.dragon",
    ),
    counter(
        "oteryn:quest-transition/native-kills/great-dragon-hunt/dragon_lord",
        "oteryn:creature.dragon_lord",
    ),
];

pub(crate) fn owns(transition: &str) -> bool {
    COUNTERS
        .iter()
        .any(|counter| counter.transition == transition)
}

pub(crate) fn admits_position(position: Option<(u16, u16, i8)>) -> bool {
    const AREAS: [(u16, u16, u16, u16, i8); 7] = [
        (33061, 32646, 33081, 32665, -6),
        (33027, 32634, 33081, 32658, -7),
        (32983, 32616, 33026, 32631, -7),
        (33007, 32612, 33020, 32623, -6),
        (32987, 32621, 33043, 32661, -6),
        (33002, 32614, 33023, 32642, -5),
        (32993, 32632, 33042, 32688, -7),
    ];
    position.is_some_and(|(x, y, floor)| {
        AREAS.iter().any(|&(x0, y0, x1, y1, z)| {
            floor == z && (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
        })
    })
}
