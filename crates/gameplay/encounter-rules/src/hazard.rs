//! Hazard / Primal Ordeal rules.
//!
//! Defaults below are pinned Canary provisional values, not verified against Global.
//! Arbitrary fixture definitions remain distinct from these production defaults.

/// One part per million. All scaling is integer and floors once at the end.
pub const PPM: u32 = 1_000_000;

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CharacterHazardLevel {
    unlocked: u16,
    selected: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HazardError {
    InvalidDefinition,
    LevelLocked,
    Overflow,
    PrimalOrdealLocked,
}

#[cfg(test)]
impl CharacterHazardLevel {
    fn new(minimum: u16) -> Result<Self, HazardError> {
        if minimum == 0 {
            return Err(HazardError::InvalidDefinition);
        }
        Ok(Self {
            unlocked: minimum,
            selected: minimum,
        })
    }

    const fn unlocked(self) -> u16 {
        self.unlocked
    }
    const fn selected(self) -> u16 {
        self.selected
    }

    /// Unlocks levels monotonically. A character's selection is never raised implicitly.
    fn unlock_through(&mut self, level: u16) {
        self.unlocked = self.unlocked.max(level);
    }

    fn select(&mut self, level: u16) -> Result<(), HazardError> {
        if level == 0 || level > self.unlocked {
            return Err(HazardError::LevelLocked);
        }
        self.selected = level;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct HazardCoefficients {
    pub creature_critical_chance_ppm: u32,
    pub creature_dodge_chance_ppm: u32,
    pub creature_damage_ppm: u32,
    pub creature_defense_ppm: u32,
    pub experience_ppm: u32,
    pub loot_chance_ppm: u32,
    pub primal_pod_spawn_chance_ppm: u32,
}

impl HazardCoefficients {
    fn validate(self) -> Result<Self, HazardError> {
        if self.creature_critical_chance_ppm > PPM
            || self.creature_dodge_chance_ppm > PPM
            || self.primal_pod_spawn_chance_ppm > PPM
            || self.creature_damage_ppm < PPM
            || self.creature_defense_ppm < PPM
            || self.experience_ppm < PPM
            || self.loot_chance_ppm < PPM
        {
            return Err(HazardError::InvalidDefinition);
        }
        Ok(self)
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct HazardAreaDefinition {
    pub key: String,
    /// Index zero is the area's minimum level. There is no extrapolation past this table.
    pub levels: Vec<HazardCoefficients>,
    /// Required selected level for entry to the Primal Ordeal boss encounter.
    pub primal_ordeal_level: u16,
}

#[cfg(test)]
impl HazardAreaDefinition {
    fn validate(&self) -> Result<(), HazardError> {
        if self.key.is_empty() || self.levels.is_empty() || self.primal_ordeal_level == 0 {
            return Err(HazardError::InvalidDefinition);
        }
        for coefficients in &self.levels {
            coefficients.validate()?;
        }
        Ok(())
    }

    fn effects(&self, level: u16) -> Result<HazardAreaEffects, HazardError> {
        self.validate()?;
        let index = usize::from(level.checked_sub(1).ok_or(HazardError::LevelLocked)?);
        let coefficients = *self.levels.get(index).ok_or(HazardError::LevelLocked)?;
        Ok(HazardAreaEffects {
            level,
            coefficients,
        })
    }

    /// Qualifies entry for the shared boss owner. The caller must use `fresh_activity=true` only
    /// after BOSS-ENCOUNTER-1 has allocated a new activity instance.
    fn enter_primal_ordeal(
        &self,
        character: CharacterHazardLevel,
        fresh_activity: bool,
    ) -> Result<PrimalOrdealEntry, HazardError> {
        self.validate()?;
        if character.selected < self.primal_ordeal_level || !fresh_activity {
            return Err(HazardError::PrimalOrdealLocked);
        }
        Ok(PrimalOrdealEntry {
            area_key: self.key.clone(),
            hazard_level: character.selected,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HazardAreaEffects {
    level: u16,
    coefficients: HazardCoefficients,
}

impl HazardAreaEffects {
    fn scale(value: u64, coefficient: u32) -> Result<u64, HazardError> {
        u128::from(value)
            .checked_mul(u128::from(coefficient))
            .and_then(|v| v.checked_div(u128::from(PPM)))
            .and_then(|v| u64::try_from(v).ok())
            .ok_or(HazardError::Overflow)
    }

    pub fn creature_damage(self, base: u64) -> Result<u64, HazardError> {
        Self::scale(base, self.coefficients.creature_damage_ppm)
    }
    #[cfg(test)]
    fn creature_defense(self, base: u64) -> Result<u64, HazardError> {
        Self::scale(base, self.coefficients.creature_defense_ppm)
    }
    pub fn experience(self, base: u64) -> Result<u64, HazardError> {
        Self::scale(base, self.coefficients.experience_ppm)
    }
    #[cfg(test)]
    fn loot_chance(self, base_ppm: u32) -> Result<u32, HazardError> {
        let scaled = Self::scale(u64::from(base_ppm), self.coefficients.loot_chance_ppm)?;
        u32::try_from(scaled.min(u64::from(PPM))).map_err(|_| HazardError::Overflow)
    }
    /// Canary applies the hazard critical first, then the ordinary hazard damage boost.
    pub fn critical_damage(self, base: u64) -> Result<u64, HazardError> {
        Self::scale(base, 1_500_000 + u32::from(self.level - 1) * 2_500)
    }
    pub fn creature_critical(self, draw_ppm: u32) -> bool {
        draw_ppm < self.coefficients.creature_critical_chance_ppm
    }
    pub fn creature_dodges(self, draw_ppm: u32) -> bool {
        draw_ppm < self.coefficients.creature_dodge_chance_ppm
    }
    #[cfg(test)]
    fn spawns_primal_pod(self, draw_ppm: u32) -> bool {
        draw_ppm < self.coefficients.primal_pod_spawn_chance_ppm
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct PrimalOrdealEntry {
    pub area_key: String,
    pub hazard_level: u16,
}

/// Canary hazard_primal.lua:3-21, inclusive bounds minus both Magma Bubble exclusions.
pub fn gnomprona_contains(x: i32, y: i32, floor: i16) -> bool {
    (33502..=33796).contains(&x)
        && (32740..=32996).contains(&y)
        && (13..=15).contains(&floor)
        && !(floor == 15
            && (((33633..=33649).contains(&x) && (32915..=32928).contains(&y))
                || ((33630..=33672).contains(&x) && (32887..=32921).contains(&y))))
}

/// Canary config.lua.dist:225-237 and player.cpp:3641-3645, 5513-5623.
/// Loot is extra independent rolls, not a multiplier on each item's chance.
pub fn provisional_effects(level: u16) -> Result<HazardAreaEffects, HazardError> {
    if !(1..=12).contains(&level) {
        return Err(HazardError::LevelLocked);
    }
    let level = u32::from(level);
    let coefficients = HazardCoefficients {
        creature_critical_chance_ppm: 75_000,
        creature_dodge_chance_ppm: level * 8_500,
        creature_damage_ppm: PPM + level * 20_000,
        creature_defense_ppm: PPM,
        experience_ppm: PPM + level * 35_000,
        loot_chance_ppm: PPM,
        primal_pod_spawn_chance_ppm: level * 8_700,
    }
    .validate()?;
    Ok(HazardAreaEffects {
        level: level as u16,
        coefficients,
    })
}

impl HazardAreaEffects {
    /// Configured loot multiplier 2 gives a 4%-per-level chance of an extra full loot roll.
    pub fn extra_loot_rolls(self, draw_percent: u32) -> u32 {
        let chance = 4 * u32::from(self.level);
        chance / 100 + u32::from(draw_percent < chance % 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hazard_canary_defaults_boundaries_and_extra_rolls() -> Result<(), HazardError> {
        assert!(gnomprona_contains(33502, 32740, 13));
        assert!(gnomprona_contains(33796, 32996, 15));
        for (x, y, z) in [
            (33501, 32740, 13),
            (33502, 32739, 13),
            (33797, 32996, 15),
            (33502, 32740, 12),
            (33640, 32920, 15),
            (33630, 32887, 15),
        ] {
            assert!(!gnomprona_contains(x, y, z));
        }
        assert!(provisional_effects(0).is_err());
        assert!(provisional_effects(13).is_err());
        for level in 1..=12 {
            let effects = provisional_effects(level)?;
            assert_eq!(effects.experience(10000)?, 10000 + u64::from(level) * 350);
            assert_eq!(
                effects.creature_damage(10000)?,
                10000 + u64::from(level) * 200
            );
            assert_eq!(
                (0..101)
                    .map(|roll| effects.extra_loot_rolls(roll))
                    .sum::<u32>(),
                u32::from(level) * 4
            );
            assert!(effects.creature_dodges(u32::from(level) * 8500 - 1));
            assert!(!effects.creature_dodges(u32::from(level) * 8500));
            assert_eq!(
                effects.critical_damage(10000)?,
                15000 + u64::from(level - 1) * 25
            );
        }
        assert_eq!(
            provisional_effects(12)?.experience(u64::MAX),
            Err(HazardError::Overflow)
        );
        Ok(())
    }

    fn fixture_area() -> HazardAreaDefinition {
        // Fixture-only numbers exercise qualification and arithmetic; they are not production data.
        HazardAreaDefinition {
            key: "fixture:crystal-cave".into(),
            levels: vec![
                HazardCoefficients {
                    creature_damage_ppm: PPM,
                    creature_defense_ppm: PPM,
                    experience_ppm: PPM,
                    loot_chance_ppm: PPM,
                    ..Default::default()
                },
                HazardCoefficients {
                    creature_critical_chance_ppm: 100_000,
                    creature_dodge_chance_ppm: 50_000,
                    creature_damage_ppm: 1_200_000,
                    creature_defense_ppm: 1_100_000,
                    experience_ppm: 1_300_000,
                    loot_chance_ppm: 1_400_000,
                    primal_pod_spawn_chance_ppm: 25_000,
                },
            ],
            primal_ordeal_level: 2,
        }
    }

    #[test]
    fn hazard_level_changes_combat_and_loot_in_fixture_area() -> Result<(), HazardError> {
        let area = fixture_area();
        let mut character = CharacterHazardLevel::new(1)?;
        assert_eq!(character.select(2), Err(HazardError::LevelLocked));
        assert_eq!(character.unlocked(), 1);
        let level_one = area.effects(character.selected())?;
        character.unlock_through(2);
        character.select(2)?;
        let level_two = area.effects(character.selected())?;
        assert_eq!(
            (
                level_one.creature_damage(100)?,
                level_one.experience(100)?,
                level_one.loot_chance(100_000)?
            ),
            (100, 100, 100_000)
        );
        assert_eq!(
            (
                level_two.creature_damage(100)?,
                level_two.experience(100)?,
                level_two.loot_chance(100_000)?
            ),
            (120, 130, 140_000)
        );
        assert_eq!(level_two.creature_defense(100)?, 110);
        assert!(level_two.creature_critical(99_999));
        assert!(level_two.creature_dodges(49_999));
        assert!(level_two.spawns_primal_pod(24_999));
        Ok(())
    }

    #[test]
    fn primal_ordeal_requires_selected_level_and_fresh_boss_activity() -> Result<(), HazardError> {
        let area = fixture_area();
        let mut character = CharacterHazardLevel::new(1)?;
        character.unlock_through(2);
        assert_eq!(
            area.enter_primal_ordeal(character, true),
            Err(HazardError::PrimalOrdealLocked)
        );
        character.select(2)?;
        assert_eq!(
            area.enter_primal_ordeal(character, false),
            Err(HazardError::PrimalOrdealLocked)
        );
        assert_eq!(area.enter_primal_ordeal(character, true)?.hazard_level, 2);
        Ok(())
    }
}
