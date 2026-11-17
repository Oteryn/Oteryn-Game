//! Crystal elder_bloodjaw.lua:104-127 and monster_ai.lua:303-401 for the Elder Bloodjaw.

use super::RoomPosition;
use oteryn_gameplay_content::content::spell_data::Vocation;

pub const ELDER_BLOODJAW_KEY: &str = "oteryn:creature.elder_bloodjaw";
/// The five bosses whose presence keeps a bloodjaw alive (sorted).
pub const ROTTEN_BLOOD_BOSSES: [&str; 5] = [
    "oteryn:creature.bakragore",
    "oteryn:creature.chagorz",
    "oteryn:creature.ichgahal",
    "oteryn:creature.murcion",
    "oteryn:creature.vemiath",
];
pub const BOSS_PRESENCE_RANGE_TILES: u16 = 15;
pub const TARGET_SEARCH_RANGE_TILES: u16 = 10;
pub const LOW_HEALTH_PERCENT: u32 = 30;
pub const LOW_HEALTH_BONUS: u16 = 30;
pub const UNKNOWN_VOCATION_PRIORITY: u16 = 50;
pub const BASE_BLOODJAWS: u8 = 2;
pub const MAX_BLOODJAWS: u8 = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BloodjawCandidate {
    pub actor: u64,
    pub position: RoomPosition,
    pub vocation: Vocation,
    pub health: u32,
    pub max_health: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NearbyMonster<'a> {
    pub creature_key: &'a str,
    pub position: RoomPosition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ElderBloodjawThink {
    /// Some(new) only when the best target differs from the current one.
    pub retarget: Option<u64>,
    pub despawn: bool,
}

/// rotten_blood_boss_mechanics.lua:53-83; `max_room_taint` is the donor taint level 0-9.
pub const fn bloodjaw_target_count(max_room_taint: u8) -> u8 {
    let mut count = BASE_BLOODJAWS;
    if max_room_taint >= 5 {
        count += 1;
    }
    if max_room_taint >= 8 {
        count += 1;
    }
    if max_room_taint >= 9 {
        count += 1;
    }
    if count > MAX_BLOODJAWS {
        MAX_BLOODJAWS
    } else {
        count
    }
}

/// monster_ai.lua:28-39; `Vocation::None` gives 50.
pub const fn vocation_priority(vocation: Vocation) -> u16 {
    match vocation {
        Vocation::Knight | Vocation::EliteKnight => 100,
        Vocation::Paladin | Vocation::RoyalPaladin => 75,
        Vocation::Druid
        | Vocation::ElderDruid
        | Vocation::Sorcerer
        | Vocation::MasterSorcerer
        | Vocation::Monk
        | Vocation::ExaltedMonk => 70,
        Vocation::None => UNKNOWN_VOCATION_PRIORITY,
    }
}

/// monster_ai.lua:93-98; `max_health == 0` counts as 100.
pub const fn health_percent(health: u32, max_health: u32) -> u32 {
    if max_health == 0 {
        100
    } else {
        ((health as u64 * 100) / max_health as u64) as u32
    }
}

pub fn is_rotten_blood_boss(creature_key: &str) -> bool {
    ROTTEN_BLOOD_BOSSES.binary_search(&creature_key).is_ok()
}

/// monster_ai.lua:303-344: highest priority within 10 tiles on the floor; ties keep the
/// earliest candidate in slice order.
pub fn best_target(own: RoomPosition, players: &[BloodjawCandidate]) -> Option<u64> {
    players
        .iter()
        .filter(|candidate| own.in_range(candidate.position, TARGET_SEARCH_RANGE_TILES))
        .map(|candidate| {
            let low_health_bonus =
                if health_percent(candidate.health, candidate.max_health) <= LOW_HEALTH_PERCENT {
                    LOW_HEALTH_BONUS
                } else {
                    0
                };
            (
                candidate,
                vocation_priority(candidate.vocation) + low_health_bonus,
            )
        })
        .reduce(|best, candidate| {
            if candidate.1 > best.1 {
                candidate
            } else {
                best
            }
        })
        .map(|(candidate, _)| candidate.actor)
}

pub fn elder_bloodjaw_think(
    own: RoomPosition,
    current_target: Option<u64>,
    players: &[BloodjawCandidate],
    monsters: &[NearbyMonster<'_>],
) -> ElderBloodjawThink {
    let retarget = best_target(own, players).filter(|best| Some(*best) != current_target);
    let despawn = !monsters.iter().any(|monster| {
        is_rotten_blood_boss(monster.creature_key)
            && own.in_range(monster.position, BOSS_PRESENCE_RANGE_TILES)
    });

    ElderBloodjawThink { retarget, despawn }
}

#[cfg(test)]
#[path = "elder_bloodjaw_think_tests.rs"]
mod tests;
