//! RANGED-1 rules (RANGED-0 §2 and §5,
//! `docs/architecture/reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md`):
//! the distance hit-chance table, the hit roll, the distance and throwing damage expression and the
//! wand bounds. Pure: the swing variant, validity, ammunition and item consequences belong to the
//! Channel owner. Every value is `PARITY_PENDING` until RANGED-PARITY-1.

use oteryn_gameplay_content::content::spell_data::formula::{
    Binary, Expression, Formula, Input, Unary,
};

/// The farthest shot the table has a row for (content rejects a `range_tiles` above 7).
pub const MAX_SHOT_DISTANCE: u32 = 7;

/// The cap of a weapon whose `max_hit_chance` is unset: 90 with an `ammo_type`, else 75 (§2).
pub const fn default_hit_chance_cap(max_hit_chance: Option<u32>, has_ammo_type: bool) -> u32 {
    match max_hit_chance {
        Some(cap) => cap,
        None if has_ammo_type => 90,
        None => 75,
    }
}

/// `trunc(min(skill, limit) * factor) + offset`, in Canary's `double` arithmetic.
#[allow(clippy::cast_possible_truncation)]
fn row(skill: u32, limit: u32, factor: f64, offset: i32) -> i32 {
    (f64::from(skill.min(limit)) * factor) as i32 + offset
}

/// The table chance at `distance` for `skill` under `cap` (`weapons.cpp:729-830`, §2): caps 75, 90
/// and 100 use their rows (distance 1 and 5 share one); any other cap is a flat chance equal to
/// the cap. None outside distances 1 to 7. Not clamped; [`shot_hit_chance`] clamps.
pub fn table_hit_chance(cap: u32, distance: u32, skill: u32) -> Option<i32> {
    if !(1..=MAX_SHOT_DISTANCE).contains(&distance) {
        return None;
    }
    let s = skill;
    Some(match (cap, distance) {
        (75, 1 | 5) => row(s, 74, 1.0, 1),
        (75, 2) => row(s, 28, 2.40, 8),
        (75, 3) => row(s, 45, 1.55, 6),
        (75, 4) => row(s, 58, 1.25, 3),
        (75, 6) => row(s, 90, 0.80, 3),
        (75, _) => row(s, 104, 0.70, 2),
        (90, 1 | 5) => row(s, 74, 1.20, 1),
        (90, 2) => row(s, 28, 3.20, 0),
        (90, 3) => row(s, 45, 2.0, 0),
        (90, 4) => row(s, 58, 1.55, 0),
        (90, _) => row(s, 90, 1.0, 0),
        (100, 1 | 5) => row(s, 73, 1.35, 1),
        (100, 2) => row(s, 30, 3.20, 4),
        (100, 3) => row(s, 48, 2.05, 2),
        (100, 4) => row(s, 65, 1.50, 2),
        (100, 6) => row(s, 87, 1.20, -4),
        (100, _) => row(s, 90, 1.10, 1),
        (flat, _) => i32::try_from(flat).ok()?,
    })
}

/// A shot's chance (ruling R6): the shot item's own nonzero `hit_chance` replaces the table and is
/// not clamped by the cap; a bow firing ammunition adds its signed `hit_chance`; the sum is clamped
/// to 0..=100. None outside distances 1 to 7.
pub fn shot_hit_chance(
    cap: u32,
    shot_absolute: Option<u32>,
    bow_modifier: Option<i32>,
    distance: u32,
    skill: u32,
) -> Option<u32> {
    let table = table_hit_chance(cap, distance, skill)?;
    let base = match shot_absolute {
        Some(absolute) if absolute > 0 => i32::try_from(absolute).ok()?,
        _ => table,
    };
    let chance = base.saturating_add(bow_modifier.unwrap_or(0)).clamp(0, 100);
    u32::try_from(chance).ok()
}

/// The one hit roll `chance >= uniform(1, 100)` over a deterministic draw.
pub fn shot_hits(chance: u32, draw: u64) -> bool {
    u64::from(chance) > draw % 100
}

/// Canary's distance damage coefficient (`weapons.cpp:899-940`).
const DISTANCE_COEFFICIENT: f64 = 0.09;

/// Distance and throwing damage (§5) over `attack_skill` (the distance skill), `attack_value` (bow
/// plus ammunition attack), `attack_factor` (the fight mode) and the official level term `L`:
///
/// ```text
/// min = floor(L)
/// max = floor(L + 0.09 * attack_factor * attack_skill * attack_value + 0.5)
/// ```
///
/// Canary rounds `max` and uses `level / 5` for `L`; the curve equals it up to level 500.
pub fn player_distance_formula() -> Formula {
    let level = || Expression::LevelBaseDamageHealing(Box::new(Expression::Var(Input::Level)));
    let mul = |left, right| Expression::Binary(Binary::Mul, Box::new(left), Box::new(right));
    let scaled = mul(
        mul(
            mul(
                Expression::Const(DISTANCE_COEFFICIENT),
                Expression::Var(Input::AttackFactor),
            ),
            Expression::Var(Input::AttackSkill),
        ),
        Expression::Var(Input::AttackValue),
    );
    let rounded = Expression::Binary(
        Binary::Add,
        Box::new(Expression::Binary(
            Binary::Add,
            Box::new(level()),
            Box::new(scaled),
        )),
        Box::new(Expression::Const(0.5)),
    );
    Formula {
        minimum: Expression::Unary(Unary::Floor, Box::new(level())),
        maximum: Expression::Unary(Unary::Floor, Box::new(rounded)),
    }
}

/// A wand or rod shot (§5): its own `damage_min..=damage_max`, no hit roll, armor or shield. None
/// when the bounds are inverted.
pub fn wand_formula(damage_min: u32, damage_max: u32) -> Option<Formula> {
    (damage_min <= damage_max).then(|| Formula {
        minimum: Expression::Const(f64::from(damage_min)),
        maximum: Expression::Const(f64::from(damage_max)),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use crate::attack::{AttackConstants, FightMode};
    use oteryn_gameplay_content::content::spell_data::formula::FormulaInputs;

    fn inputs(level: u32, skill: u32, attack: u32, mode: FightMode) -> FormulaInputs {
        let table = AttackConstants::checked_in().expect("the checked-in constants table parses");
        FormulaInputs {
            level,
            magic_level: 0,
            base_power: None,
            attack_skill: skill,
            attack_value: attack,
            attack_factor: table.fight_mode(mode).attack_factor,
            shielding_skill: 0,
            shield_defense: None,
        }
    }

    #[test]
    fn the_table_rows_reach_the_decisions_ceilings() {
        // §2 ceilings at the row limits, distances 1, 2, 3, 4, 6, 7.
        let ceilings = [
            (75, [75, 75, 75, 75, 75, 74]),
            (90, [89, 89, 90, 89, 90, 90]),
            (100, [99, 100, 100, 99, 100, 100]),
        ];
        for (cap, expected) in ceilings {
            let got: Vec<i32> = [1, 2, 3, 4, 6, 7]
                .iter()
                .map(|&d| table_hit_chance(cap, d, 200).unwrap())
                .collect();
            assert_eq!(got, expected, "cap {cap}");
            // Distance 5 shares distance 1's row.
            assert_eq!(
                table_hit_chance(cap, 5, 37),
                table_hit_chance(cap, 1, 37),
                "cap {cap}"
            );
        }
    }

    #[test]
    fn low_skill_rows_and_flat_caps() {
        assert_eq!(table_hit_chance(75, 2, 10), Some(32));
        assert_eq!(table_hit_chance(90, 4, 20), Some(31));
        assert_eq!(table_hit_chance(100, 6, 0), Some(-4));
        // A cap with no rows is flat at every distance.
        for d in 1..=7 {
            assert_eq!(table_hit_chance(76, d, 0), Some(76));
        }
        assert_eq!(table_hit_chance(75, 0, 50), None);
        assert_eq!(table_hit_chance(75, 8, 50), None);
    }

    #[test]
    fn an_absolute_chance_replaces_the_table_and_a_bow_adds_its_modifier() {
        // Throwing item with its own chance: the table is not read and the cap does not clamp.
        assert_eq!(shot_hit_chance(86, Some(90), None, 3, 0), Some(90));
        // Zero means the table.
        assert_eq!(shot_hit_chance(75, Some(0), None, 2, 10), Some(32));
        // A bow's modifier is added to the table and the sum is clamped.
        assert_eq!(shot_hit_chance(90, None, Some(5), 6, 90), Some(95));
        assert_eq!(shot_hit_chance(100, None, Some(10), 2, 200), Some(100));
        assert_eq!(shot_hit_chance(100, None, None, 6, 0), Some(0));
        assert_eq!(shot_hit_chance(75, None, Some(-100), 1, 74), Some(0));
        assert_eq!(default_hit_chance_cap(None, true), 90);
        assert_eq!(default_hit_chance_cap(None, false), 75);
        assert_eq!(default_hit_chance_cap(Some(76), true), 76);
    }

    #[test]
    fn the_hit_roll_is_chance_out_of_one_hundred() {
        assert!(!shot_hits(0, 0));
        assert!(shot_hits(1, 0));
        assert!(!shot_hits(1, 1));
        assert!(shot_hits(100, 99));
        assert!(shot_hits(50, 149));
        assert!(!shot_hits(50, 150));
        let hits = (0..100u64).filter(|&draw| shot_hits(76, draw)).count();
        assert_eq!(hits, 76);
    }

    #[test]
    fn distance_damage_rounds_canarys_maximum_over_the_level_term() {
        let formula = player_distance_formula();
        // Level 100, distance 80, attack 25 (spear), offensive: 0.09 * 80 * 25 = 180 over L = 20.
        assert_eq!(
            formula.bounds(&inputs(100, 80, 25, FightMode::Offensive)),
            Ok((20, 200))
        );
        // Level 8, distance 11, attack 25: L = 1, 1 + 0.09 * 275 = 25.75 rounds up to 26.
        assert_eq!(
            formula.bounds(&inputs(8, 11, 25, FightMode::Offensive)),
            Ok((1, 26))
        );
    }

    #[test]
    fn a_wand_draws_within_its_own_bounds() {
        let formula = wand_formula(8, 18).unwrap();
        assert_eq!(
            formula.bounds(&inputs(1, 0, 0, FightMode::Defensive)),
            Ok((8, 18))
        );
        assert!(wand_formula(9, 8).is_none());
    }
}
