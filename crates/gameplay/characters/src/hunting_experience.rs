//! D118's closed modifier order: a Character's party/damage share, Prey, then stamina.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidHuntingExperience;

pub fn hunting_experience(
    share: i64,
    prey_percent: i16,
    stamina_minutes: i32,
    premium: bool,
) -> Result<i64, InvalidHuntingExperience> {
    if share < 0 || !(0..=100).contains(&prey_percent) || !(0..=2520).contains(&stamina_minutes) {
        return Err(InvalidHuntingExperience);
    }
    let prey = i128::from(share) * (100 + i128::from(prey_percent)) / 100;
    let award = if stamina_minutes == 0 {
        0
    } else if stamina_minutes <= 840 {
        prey / 2
    } else if stamina_minutes > 2340 && premium {
        prey * 3 / 2
    } else {
        prey
    };
    i64::try_from(award).map_err(|_| InvalidHuntingExperience)
}

#[cfg(test)]
#[path = "hunting_experience_tests.rs"]
mod tests;
