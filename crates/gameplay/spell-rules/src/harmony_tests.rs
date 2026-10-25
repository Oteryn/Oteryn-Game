//! Pure Harmony multiplier regression tests.
#![allow(clippy::expect_used)]

use super::harmony::{HarmonyMultiplier, MULTIPLIER_DENOMINATOR};

#[test]
fn the_harmony_multiplier_matches_the_engine_tests() {
    let ratio = |level, charges, virtue, serene| {
        HarmonyMultiplier::new(level, charges, virtue, serene)
            .expect("charges")
            .numerator()
    };
    let hundredths = |values: [u64; 5]| values.map(|value| value * MULTIPLIER_DENOMINATOR / 100);
    // Level 200 without a virtue (Serene does not matter): 1.08, 1.16, 1.32, 1.64, 2.28.
    for serene in [false, true] {
        assert_eq!(
            [1, 2, 3, 4, 5].map(|charges| ratio(200, charges, false, serene)),
            hundredths([108, 116, 132, 164, 228])
        );
    }
    // Virtue of Harmony while Serene: 1.16, 1.32, 1.64, 2.28, 3.56.
    assert_eq!(
        [1, 2, 3, 4, 5].map(|charges| ratio(200, charges, true, true)),
        hundredths([116, 132, 164, 228, 356])
    );
    // Virtue without Serene, 5 charges: 2.92; level 700 with virtue and Serene: 4.36.
    assert_eq!(
        ratio(200, 5, true, false),
        292 * MULTIPLIER_DENOMINATOR / 100
    );
    assert_eq!(
        ratio(700, 5, true, true),
        436 * MULTIPLIER_DENOMINATOR / 100
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
    // x1.32: 37 -> 48.84 -> 48; 45 -> 59.4 -> 59; negative bounds truncate toward zero.
    assert_eq!(multiplier.apply(37), 48);
    assert_eq!(multiplier.apply(45), 59);
    assert_eq!(multiplier.apply(-37), -48);
    assert_eq!(multiplier.apply(0), 0);
    assert_eq!(HarmonyMultiplier::ONE.apply(37), 37);
}
