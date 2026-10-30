//! RANGED-1 rules (RANGED-0 §2 and §5,
//! `docs/architecture/reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md`):
//! the distance hit-chance table, the hit roll, the distance and throwing damage expression and the
//! wand bounds. Pure: the swing variant, validity, ammunition and item consequences belong to the
//! Channel owner. AUTOATK-1 pins Offensive raw damage bounds; hit chance, the damage
//! distribution and other fight modes remain `PARITY_PENDING`.

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

/// Distance and throwing raw bounds from TibiaTools `fe745abf`:
/// `damage-calc/calc.ts:102,132` calls `damage.ts:13-26`. Bows and crossbows add
/// ammunition attack to launcher attack (`character-state.ts:75`); throwing weapons use
/// their own attack. The distance skill is `attack_skill` and their total attack is `attack_value`.
///
/// ```text
/// v = floor(6 * attack_value / 5) * (attack_skill + 4) / 28 * attack_factor
/// min = floor(L + v / 2)
/// max = floor(L + 2 * v)
/// ```
///
/// `L` is the official level curve. The API fixtures bind Offensive (factor 1.0);
/// other fight modes scale `v` and remain `PARITY_PENDING`.
pub fn player_distance_formula() -> Formula {
    let level = || Expression::LevelBaseDamageHealing(Box::new(Expression::Var(Input::Level)));
    let binary = |op, left, right| Expression::Binary(op, Box::new(left), Box::new(right));
    let floor = |inner| Expression::Unary(Unary::Floor, Box::new(inner));
    let attack = floor(binary(
        Binary::Div,
        binary(
            Binary::Mul,
            Expression::Const(6.0),
            Expression::Var(Input::AttackValue),
        ),
        Expression::Const(5.0),
    ));
    let skill = binary(
        Binary::Add,
        Expression::Var(Input::AttackSkill),
        Expression::Const(4.0),
    );
    let v = binary(
        Binary::Mul,
        binary(
            Binary::Div,
            binary(Binary::Mul, attack, skill),
            Expression::Const(28.0),
        ),
        Expression::Var(Input::AttackFactor),
    );
    Formula {
        minimum: floor(binary(
            Binary::Add,
            level(),
            binary(Binary::Div, v.clone(), Expression::Const(2.0)),
        )),
        maximum: floor(binary(
            Binary::Add,
            level(),
            binary(Binary::Mul, v, Expression::Const(2.0)),
        )),
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
    fn distance_damage_uses_the_calculator_bounds_over_the_level_term() {
        let formula = player_distance_formula();
        // Level 100 / distance 80 / spear 25: v = 90 over L = 20.
        // Minimum changes 20 -> 65 because the calculator includes v / 2; max stays 200.
        assert_eq!(
            formula.bounds(&inputs(100, 80, 25, FightMode::Offensive)),
            Ok((65, 200))
        );
        // Level 8 / distance 11 / spear 25: L = 1, v = 450 / 28.
        // (1, 26) -> (9, 33): include v / 2 in min and floor L + 2*v in max.
        assert_eq!(
            formula.bounds(&inputs(8, 11, 25, FightMode::Offensive)),
            Ok((9, 33))
        );
    }

    #[test]
    fn autoatk_paladin_bow_matches_saved_api_responses() {
        let formula = player_distance_formula();
        // Bow (184) + Arrow (1), total attack 25. Evidence: research/autoatk/.
        for (level, skill, expected, response) in [
            (300, 80, (105, 240), "paladin-bow-300.response.json"),
            (500, 100, (155, 322), "paladin-bow-500.response.json"),
            (1000, 120, (249, 448), "paladin-bow-1000.response.json"),
        ] {
            assert_eq!(
                formula.bounds(&inputs(level, skill, 25, FightMode::Offensive)),
                Ok(expected),
                "{response}"
            );
        }
    }

    #[test]
    fn autoatk_paladin_spear_matches_saved_api_responses() {
        let formula = player_distance_formula();
        // Royal Spear (717), attack 35. Evidence: research/autoatk/.
        for (level, skill, expected, response) in [
            (300, 80, (123, 312), "paladin-spear-300.response.json"),
            (500, 100, (178, 412), "paladin-spear-500.response.json"),
            (1000, 120, (276, 555), "paladin-spear-1000.response.json"),
        ] {
            assert_eq!(
                formula.bounds(&inputs(level, skill, 35, FightMode::Offensive)),
                Ok(expected),
                "{response}"
            );
        }
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
