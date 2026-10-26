//! PARITY-EQUIP-MODIFIERS-1: the critical hit and leech of the worn equipment, in Canary's
//! hundredths of a percent (10000 = 100%).

/// The roll range of a critical hit draw (`combat.cpp` `uniform_random(1, 10000)`).
pub const CRITICAL_ROLL_MIN: u64 = 1;
pub const CRITICAL_ROLL_MAX: u64 = 10_000;

/// The worn equipment combat projection. Critical/leech skills use `[0, u16::MAX]`
/// as `Player::getSkillLevel` returns them; the other attributes retain their own units.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EquippedOffence {
    pub critical_chance: u16,
    pub critical_damage: u16,
    pub life_leech: u16,
    pub mana_leech: u16,
    /// EQMOD2: hundredths of a percent, before defence; extension hits never crit/leech.
    pub cleave: u64,
    /// Canary item.cpp:571-583,607-619: installed weapon and boots tiers.
    pub weapon_tier: u8,
    pub boots_tier: u8,
    pub reflect_percent: u64,
    /// Physical reflection points; other elements receive only reflectpercentall.
    pub reflect_flat: u32,
    /// Flat ranged bonus at each exact axis distance, after the target's block.
    pub perfect_shot: std::collections::BTreeMap<u16, i64>,
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
            ..Self::default()
        }
    }

    /// Whether a swing draws the critical roll at all (`combat.cpp:2724`: no chance, no roll).
    #[must_use]
    pub const fn rolls_critical(&self) -> bool {
        self.critical_chance != 0
    }

    /// The damage after a critical hit check (`Combat::applyExtensions`, `combat.cpp:2724-2735`):
    /// a roll at most the chance multiplies by `1 + bonus / 10000`, truncated.
    #[must_use]
    pub fn critical(&self, damage: u32, roll: u64) -> u32 {
        if !self.rolls_critical() || roll > u64::from(self.critical_chance) {
            return damage;
        }
        let multiplied = u64::from(damage) * (10_000 + u64::from(self.critical_damage)) / 10_000;
        u32::try_from(multiplied).unwrap_or(u32::MAX)
    }

    /// Pinned Canary config.lua.dist:195-209, combat.cpp:2674-2681,2752-2756.
    /// Boots amplify the fatal chance, not critical chance or base damage. Roll 0..=10000.
    #[must_use]
    pub fn fatal(&self, damage: u32, roll: u64) -> u32 {
        if self.weapon_tier == 0 {
            return damage;
        }
        let weapon = u64::from(self.weapon_tier.min(10));
        let boots = u64::from(self.boots_tier.min(10));
        let base = 5 * weapon * weapon + 40 * weapon + 5;
        let amplification = if boots == 0 {
            0
        } else {
            40 * boots * boots + 170 * boots + 40
        };
        if roll.saturating_mul(10_000) >= base * (10_000 + amplification) {
            return damage;
        }
        // The signed source uses round; magnitude is positive here.
        damage
            .saturating_add(u32::try_from((u64::from(damage) * 60 + 50) / 100).unwrap_or(u32::MAX))
    }

    /// Canary game.cpp:7962-7966: floor the percent of positive damage (ceil of
    /// its negative), add the flat, then cap at ceil(attacker maximum health / 100).
    /// Physical flat alone is melee range only; percent reflection works at any distance.
    #[must_use]
    pub fn reflected(&self, damage: u32, physical: bool, distance: u32, maximum: u64) -> u32 {
        let flat = if physical { self.reflect_flat } else { 0 };
        if physical && self.reflect_percent == 0 && distance >= 2 {
            return 0;
        }
        let percent = u128::from(damage) * u128::from(self.reflect_percent) / 10_000;
        u32::try_from((percent + u128::from(flat)).min(u128::from(maximum.div_ceil(100))))
            .unwrap_or(u32::MAX)
    }

    /// Canary game.cpp:8494-8526: axis matches add, a diagonal matches only once.
    #[must_use]
    pub fn perfect_damage(&self, dx: u32, dy: u32) -> i64 {
        let at = |d| {
            u16::try_from(d)
                .ok()
                .and_then(|d| self.perfect_shot.get(&d))
                .copied()
                .unwrap_or(0)
        };
        at(dx).saturating_add(if dx == dy { 0 } else { at(dy) })
    }
}

/// The leech of one hit (`Game::calculateLeechAmount`, `game.cpp:9058-9061`) on its single
/// target: `round(real * skill / 10000)`, never above the damage dealt.
#[must_use]
pub fn leech_amount(real_damage: u32, skill: u16) -> u32 {
    let amount = (u64::from(real_damage) * u64::from(skill) + 5_000) / 10_000;
    u32::try_from(amount.min(u64::from(real_damage))).unwrap_or(real_damage)
}

/// Canary weapons.cpp:541-561: two neighbours flank the target, never the attacker.
#[must_use]
pub fn cleave_tiles(from: (i32, i32), target: (i32, i32)) -> Option<[(i32, i32); 2]> {
    let (x, y) = target;
    if from == target {
        return None;
    }
    Some(if x == from.0 {
        [(x.checked_add(1)?, y), (x.checked_sub(1)?, y)]
    } else if y == from.1 {
        [(x, y.checked_add(1)?), (x, y.checked_sub(1)?)]
    } else {
        [
            (x.checked_add(if x > from.0 { -1 } else { 1 })?, y),
            (x, y.checked_add(if y > from.1 { -1 } else { 1 })?),
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eqmod2_reflection_flat_range_percent_floor_and_health_cap() {
        let mut offence = EquippedOffence {
            reflect_flat: 5,
            ..EquippedOffence::default()
        };
        assert_eq!(offence.reflected(100, true, 1, 1000), 5);
        assert_eq!(offence.reflected(100, true, 2, 1000), 0);
        assert_eq!(offence.reflected(100, false, 1, 1000), 0);
        offence.reflect_percent = 1250;
        assert_eq!(offence.reflected(15, false, 8, 1000), 1);
        assert_eq!(offence.reflected(100, true, 8, 1050), 11);
        assert_eq!(offence.reflected(0, true, 1, 1000), 5);
    }

    #[test]
    fn eqmod2_perfect_shot_counts_axis_matches_once_on_a_diagonal() {
        let offence = EquippedOffence {
            perfect_shot: [(3, 20), (4, 30)].into(),
            ..EquippedOffence::default()
        };
        assert_eq!(offence.perfect_damage(3, 3), 20);
        assert_eq!(offence.perfect_damage(3, 4), 50);
        assert_eq!(offence.perfect_damage(2, 3), 20);
        assert_eq!(offence.perfect_damage(2, 2), 0);
    }

    #[test]
    fn eqmod2_cleave_flanks_cardinal_and_diagonal_targets() {
        assert_eq!(cleave_tiles((0, 0), (1, 0)), Some([(1, 1), (1, -1)]));
        assert_eq!(cleave_tiles((0, 0), (0, -1)), Some([(1, -1), (-1, -1)]));
        assert_eq!(cleave_tiles((0, 0), (1, 1)), Some([(0, 1), (1, 0)]));
        assert_eq!(cleave_tiles((0, 0), (-1, -1)), Some([(0, -1), (-1, 0)]));
        assert_eq!(cleave_tiles((0, 0), (0, 0)), None);
        assert_eq!(cleave_tiles((i32::MAX, 0), (i32::MAX, 1)), None);
    }

    #[test]
    fn eqmod2_boots_amplify_fatal_chance_after_critical_damage() {
        let mut offence = EquippedOffence {
            weapon_tier: 1,
            ..EquippedOffence::default()
        };
        assert_eq!(offence.fatal(11, 49), 18);
        assert_eq!(offence.fatal(11, 50), 11);
        offence.boots_tier = 1; // 0.5% * 1.025 = 0.5125%.
        assert_eq!(offence.fatal(11, 51), 18);
        assert_eq!(offence.fatal(11, 52), 11);
        offence.weapon_tier = 0;
        assert_eq!(offence.fatal(11, 0), 11);
    }

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
                ..EquippedOffence::default()
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
