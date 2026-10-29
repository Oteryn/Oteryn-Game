//! Pure Harmony multiplier regression tests.
#![allow(clippy::expect_used)]

use super::harmony::{HarmonyMultiplier, MULTIPLIER_DENOMINATOR};

#[test]
fn global_harmony_adds_the_level_bonus_once_then_scales_by_virtue() {
    let ratio = |level, charges, virtue, serene| {
        HarmonyMultiplier::new(level, charges, virtue, serene)
            .expect("charges")
            .numerator()
    };
    let hundredths = |values: [u64; 5]| values.map(|value| value * MULTIPLIER_DENOMINATOR / 100);
    // Global: level 200 adds 1% to each entry of 7/14/28/56/112, once.
    for serene in [false, true] {
        assert_eq!(
            [1, 2, 3, 4, 5].map(|charges| ratio(200, charges, false, serene)),
            hundredths([108, 115, 129, 157, 213])
        );
    }
    // Virtue of Harmony while Serene doubles the complete bonus.
    assert_eq!(
        [1, 2, 3, 4, 5].map(|charges| ratio(200, charges, true, true)),
        hundredths([116, 130, 158, 214, 326])
    );
    // Virtue without Serene: 113% x1.5; level 700 and Serene: 115.5% x2.
    assert_eq!(
        ratio(200, 5, true, false),
        2695 * MULTIPLIER_DENOMINATOR / 1000
    );
    assert_eq!(
        ratio(700, 5, true, true),
        331 * MULTIPLIER_DENOMINATOR / 100
    );
    // No charges: 1; more than 5 cannot be held.
    assert_eq!(
        HarmonyMultiplier::new(200, 0, true, true),
        Some(HarmonyMultiplier::ONE)
    );
    assert_eq!(HarmonyMultiplier::new(200, 6, false, false), None);
}

#[test]
fn a_spender_bound_is_multiplied_and_then_truncated_toward_zero() {
    let multiplier = HarmonyMultiplier::new(200, 3, false, false).expect("charges");
    // x1.29: 37 -> 47.73 -> 47; 45 -> 58.05 -> 58; negative bounds truncate toward zero.
    assert_eq!(multiplier.apply(37), 47);
    assert_eq!(multiplier.apply(45), 58);
    assert_eq!(multiplier.apply(-37), -47);
    assert_eq!(multiplier.apply(0), 0);
    assert_eq!(HarmonyMultiplier::ONE.apply(37), 37);
}
