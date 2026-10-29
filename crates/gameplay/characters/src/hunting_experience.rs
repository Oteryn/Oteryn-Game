//! D118's closed modifier order: a Character's party/damage share, Prey, daily XP boost plus the
//! low-level bonus, then stamina.
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
    if !matches!(xp_boost_percent, 0 | 50)
        || share < 0
        || !(0..=100).contains(&prey_percent)
        || !(0..=2520).contains(&stamina_minutes)
    {
        return Err(InvalidHuntingExperience);
    }
    let prey = i128::from(share) * (100 + i128::from(prey_percent)) / 100;
    // Additive with the daily boost, as Crystal and Canary add `boost` and `lowLevel` percents.
    let boosted =
        prey * (1200 + 12 * i128::from(xp_boost_percent) + low_level_bonus_1200ths(level)) / 1200;
    let award = if stamina_minutes == 0 {
        0
    } else if stamina_minutes <= 840 {
        boosted / 2
    } else if stamina_minutes > 2340 && premium {
        boosted * 3 / 2
    } else {
        boosted
    };
    i64::try_from(award).map_err(|_| InvalidHuntingExperience)
}

#[cfg(test)]
#[path = "hunting_experience_tests.rs"]
mod tests;
