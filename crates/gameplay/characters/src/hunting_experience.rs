//! D118's closed modifier order: a Character's party/damage share, Prey, daily XP boost, then stamina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidHuntingExperience;

pub fn hunting_experience(
    share: i64,
    prey_percent: i16,
    xp_boost_percent: i16,
    stamina_minutes: i32,
    premium: bool,
) -> Result<i64, InvalidHuntingExperience> {
    if !matches!(xp_boost_percent, 0 | 50)
        || share < 0
        || !(0..=100).contains(&prey_percent)
        || !(0..=2520).contains(&stamina_minutes)
    {
        return Err(InvalidHuntingExperience);
    }
    let prey = i128::from(share) * (100 + i128::from(prey_percent)) / 100;
    let boosted = prey * (100 + i128::from(xp_boost_percent)) / 100;
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

/// Animus multiplies the raw damage/party share independently of the other hunting bonuses.
/// Preserve its fractional part through the multipliers and round once at the final award.
/// Unmastered creatures retain the existing hunting owner's rounding behavior.
pub fn hunting_experience_with_animus(
    share: i64,
    prey_percent: i16,
    xp_boost_percent: i16,
    stamina_minutes: i32,
    premium: bool,
    animus_basis_points: i16,
) -> Result<i64, InvalidHuntingExperience> {
    let ordinary = hunting_experience(
        share,
        prey_percent,
        xp_boost_percent,
        stamina_minutes,
        premium,
    )?;
    if animus_basis_points == 0 {
        return Ok(ordinary);
    }
    if !(200..=400).contains(&animus_basis_points) || animus_basis_points % 10 != 0 {
        return Err(InvalidHuntingExperience);
    }
    let mut numerator = i128::from(share)
        * (10_000 + i128::from(animus_basis_points))
        * (100 + i128::from(prey_percent))
        * (100 + i128::from(xp_boost_percent));
    let mut denominator = 100_000_000_i128;
    if stamina_minutes == 0 {
        return Ok(0);
    } else if stamina_minutes <= 840 {
        denominator *= 2;
    } else if stamina_minutes > 2340 && premium {
        numerator *= 3;
        denominator *= 2;
    }
    i64::try_from(numerator / denominator).map_err(|_| InvalidHuntingExperience)
}

#[cfg(test)]
#[path = "hunting_experience_tests.rs"]
mod tests;
