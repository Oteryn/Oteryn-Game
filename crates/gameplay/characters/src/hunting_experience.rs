//! Global modifier order: a Character's party/damage share, Prey, daily XP boost plus the
//! low-level bonus, then stamina, with integer conversion once at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidHuntingExperience;

/// Global low-level bonus in 1/1200 units of the kill XP: 100% up to level 2, then down by
/// 2 1/12 percentage points per level to 0% at level 50 (TibiaWiki Experience Point, rev 1161628).
pub fn low_level_bonus_1200ths(level: u32) -> i128 {
    let steps = level.clamp(2, 50) - 2;
    1200 - 25 * i128::from(steps)
}

pub fn hunting_experience(
    share: i64,
    prey_percent: i16,
    xp_boost_percent: i16,
    stamina_minutes: i32,
    premium: bool,
    level: u32,
) -> Result<i64, InvalidHuntingExperience> {
    if !matches!(xp_boost_percent, 0 | 10 | 20 | 25 | 50)
        || share < 0
        || !(0..=100).contains(&prey_percent)
        || !(0..=2520).contains(&stamina_minutes)
    {
        return Err(InvalidHuntingExperience);
    }
    // Owner C15 adopts Global single final truncation for all modifiers. Crystal 201189f
    // data/events/scripts/player.lua:523 ceils Prey first; :627 combines the remaining factors.
    let numerator = i128::from(share)
        * (100 + i128::from(prey_percent))
        * (1200 + 12 * i128::from(xp_boost_percent) + low_level_bonus_1200ths(level));
    let (stamina_numerator, stamina_denominator) = if stamina_minutes == 0 {
        (0, 1)
    } else if stamina_minutes <= 840 {
        (1, 2)
    } else if stamina_minutes > 2340 && premium {
        (3, 2)
    } else {
        (1, 1)
    };
    let award = numerator * stamina_numerator / (100 * 1200 * stamina_denominator);
    i64::try_from(award).map_err(|_| InvalidHuntingExperience)
}

#[cfg(test)]
#[path = "hunting_experience_tests.rs"]
mod tests;
