//! Pure Harmony multiplier regression tests.
#![allow(clippy::expect_used)]

use super::harmony::{HarmonyMultiplier, MULTIPLIER_DENOMINATOR};

#[test]
fn global_harmony_scales_the_complete_base_per_charge() {
    let ratio = |level, charges, virtue, serene| {
        HarmonyMultiplier::new(level, charges, virtue, serene)
            .expect("charges")
            .numerator()
    };
    let hundredths = |values: [u64; 5]| values.map(|value| value * MULTIPLIER_DENOMINATOR / 100);
    // C11PKG decision 1: level and Ascetic belong inside the base before charge scaling.
    for serene in [false, true] {
        assert_eq!(
            [1, 2, 3, 4, 5].map(|charges| ratio(200, charges, false, serene)),
            hundredths([108, 116, 132, 164, 228])
        );
    }
    // Virtue of Harmony while Serene doubles the complete bonus.
    assert_eq!(
        [1, 2, 3, 4, 5].map(|charges| ratio(200, charges, true, true)),
        hundredths([116, 132, 164, 228, 356])
    );
    // Virtue scales (7 + level * 0.005) before the five-charge factor 16.
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
    // x1.32: 37 -> 48.84 -> 48; 45 -> 59.4 -> 59; truncate toward zero.
    assert_eq!(multiplier.apply(37), 48);
    assert_eq!(multiplier.apply(45), 59);
    assert_eq!(multiplier.apply(-37), -48);
    assert_eq!(multiplier.apply(0), 0);
    assert_eq!(HarmonyMultiplier::ONE.apply(37), 37);
}

#[test]
fn ascetic_is_inside_the_base_and_the_virtue_and_charge_factors() {
    for (virtue, serene, percent) in [(true, true, 452), (false, true, 276), (true, false, 364)] {
        let m = HarmonyMultiplier::with_ascetic(200, 5, virtue, serene, 3).expect("stage");
        assert_eq!(m.numerator(), percent * MULTIPLIER_DENOMINATOR / 100);
        assert_eq!(
            m.without_ascetic(),
            HarmonyMultiplier::new(200, 5, virtue, serene).expect("stage")
        );
    }
    assert!(HarmonyMultiplier::with_ascetic(200, 5, true, true, 4).is_none());
    assert_eq!(
        HarmonyMultiplier::with_ascetic(200, 0, true, true, 3),
        Some(HarmonyMultiplier::ONE)
    );
}
