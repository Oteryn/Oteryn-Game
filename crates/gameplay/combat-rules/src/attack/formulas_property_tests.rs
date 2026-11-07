#![allow(clippy::expect_used)]
use super::{AttackConstants, FightMode, formulas};
use oteryn_gameplay_content::content::spell_data::formula::{
    FormulaInputs, level_base_damage_healing,
};
use oteryn_test_support::property::for_each_case;

#[test]
fn property_attack_bounds_are_nonnegative_and_monotone_in_level_and_skill() {
    let constants = AttackConstants::checked_in().expect("checked-in constants");
    let formulas = [
        formulas::player_fist_formula(constants),
        formulas::player_melee_formula(constants),
        formulas::creature_melee_formula(constants),
    ];
    for_each_case("attack monotonicity", |rng, _| {
        let level = rng.below(100_001) as u32;
        let skill = rng.below(1001) as u32;
        let input = FormulaInputs {
            level,
            magic_level: 0,
            base_power: None,
            attack_skill: skill,
            attack_value: rng.below(1001) as u32,
            attack_factor: constants.fight_mode(FightMode::Offensive).attack_factor,
            shielding_skill: 0,
            shield_defense: None,
        };
        for formula in &formulas {
            let bounds = formula.bounds(&input).expect("representable damage");
            assert!(0 <= bounds.0 && bounds.0 <= bounds.1);
            for larger in [
                FormulaInputs {
                    level: level + 1,
                    ..input
                },
                FormulaInputs {
                    attack_skill: skill + 1,
                    ..input
                },
            ] {
                let raised = formula.bounds(&larger).expect("representable damage");
                assert!(raised.0 >= bounds.0 && raised.1 >= bounds.1);
            }
        }
        // Full u32 input range and the curve's scaling boundaries; the integer
        // level term itself has no i32 magnitude restriction.
        for level in [
            0,
            499,
            500,
            501,
            1099,
            1100,
            1101,
            1799,
            1800,
            1801,
            rng.u64() as u32,
            u32::MAX - 1,
        ] {
            let value = level_base_damage_healing(level);
            assert!(value >= 0);
            assert!(level_base_damage_healing(level.saturating_add(1)) >= value);
        }
        let extreme = FormulaInputs {
            attack_skill: u32::MAX,
            attack_value: u32::MAX,
            ..input
        };
        for formula in &formulas[1..] {
            assert!(
                formula.bounds(&extreme).is_err(),
                "unrepresentable damage must refuse"
            );
        }
        Ok(())
    });
}

#[test]
fn property_armor_bounds_are_ordered_bounded_and_monotone() {
    let constants = AttackConstants::checked_in().expect("checked-in constants");
    for_each_case("armor bounds", |rng, _| {
        for armor in [0, 1, 2, 3, rng.u64() as u32, u32::MAX] {
            let (low, high) = formulas::armor_reduction_bounds(constants, armor);
            assert!(low <= high && high <= armor);
            if armor < u32::MAX {
                let next = formulas::armor_reduction_bounds(constants, armor + 1);
                assert!(next.0 >= low && next.1 >= high);
            }
        }
        Ok(())
    });
}
