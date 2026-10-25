//! PARITY-EQUIP-MODIFIERS-1: the critical hit and leech of the worn equipment, in Canary's
//! hundredths of a percent (10000 = 100%).

/// The roll range of a critical hit draw (`combat.cpp` `uniform_random(1, 10000)`).
pub const CRITICAL_ROLL_MIN: u64 = 1;
pub const CRITICAL_ROLL_MAX: u64 = 10_000;

/// The summed offence skills of the worn equipment, each clamped to `[0, u16::MAX]` as
/// `Player::getSkillLevel` returns them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EquippedOffence {
    pub critical_chance: u16,
    pub critical_damage: u16,
    pub life_leech: u16,
    pub mana_leech: u16,
}

impl EquippedOffence {
    /// Clamp signed per-skill sums into the skill range.
    #[must_use]
    pub fn from_sums(
        critical_chance: i64,
        critical_damage: i64,
        life_leech: i64,
        mana_leech: i64,
    ) -> Self {
        let skill = |sum: i64| u16::try_from(sum.clamp(0, i64::from(u16::MAX))).unwrap_or(0);
        Self {
            critical_chance: skill(critical_chance),
            critical_damage: skill(critical_damage),
            life_leech: skill(life_leech),
            mana_leech: skill(mana_leech),
        }
    }

    /// Whether a swing draws the critical roll at all (`combat.cpp:2724`: no chance, no roll).
    #[must_use]
    pub const fn rolls_critical(self) -> bool {
        self.critical_chance != 0
    }

    /// The damage after a critical hit check (`Combat::applyExtensions`, `combat.cpp:2724-2735`):
    /// a roll at most the chance multiplies by `1 + bonus / 10000`, truncated.
    #[must_use]
    pub fn critical(self, damage: u32, roll: u64) -> u32 {
        if !self.rolls_critical() || roll > u64::from(self.critical_chance) {
            return damage;
        }
        let multiplied = u64::from(damage) * (10_000 + u64::from(self.critical_damage)) / 10_000;
        u32::try_from(multiplied).unwrap_or(u32::MAX)
    }
}

/// The leech of one hit (`Game::calculateLeechAmount`, `game.cpp:9058-9061`) on its single
/// target: `round(real * skill / 10000)`, never above the damage dealt.
#[must_use]
pub fn leech_amount(real_damage: u32, skill: u16) -> u32 {
    let amount = (u64::from(real_damage) * u64::from(skill) + 5_000) / 10_000;
    u32::try_from(amount.min(u64::from(real_damage))).unwrap_or(real_damage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_clamp_to_the_skill_range() {
        let offence = EquippedOffence::from_sums(-5, 70_000, 600, 0);
        assert_eq!(
            offence,
            EquippedOffence {
                critical_chance: 0,
                critical_damage: u16::MAX,
                life_leech: 600,
                mana_leech: 0,
            }
        );
        assert!(!offence.rolls_critical());
    }

    #[test]
    fn a_roll_at_most_the_chance_multiplies_the_damage() {
        let offence = EquippedOffence {
            critical_chance: 500,
            critical_damage: 2500,
            ..EquippedOffence::default()
        };
        assert_eq!(offence.critical(40, 500), 50, "the boundary roll is a hit");
        assert_eq!(offence.critical(40, 1), 50);
        assert_eq!(offence.critical(40, 501), 40);
        assert_eq!(offence.critical(41, 1), 51, "the product is truncated");
        assert_eq!(EquippedOffence::default().critical(40, 1), 40);
    }

    #[test]
    fn leech_rounds_and_never_exceeds_the_damage() {
        assert_eq!(leech_amount(100, 1800), 18);
        assert_eq!(leech_amount(25, 200), 1, "0.5 rounds half away from zero");
        assert_eq!(leech_amount(24, 200), 0);
        assert_eq!(leech_amount(10, u16::MAX), 10);
        assert_eq!(leech_amount(0, 1800), 0);
    }
}
