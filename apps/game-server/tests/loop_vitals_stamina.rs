//! Tests the one pure stamina arithmetic leaf used by the native durability owner.

#[path = "../src/durability/character_vitals_stamina_math.rs"]
mod character_vitals_stamina;

use character_vitals_stamina::{
    MAX_OFFLINE_SECONDS, MAX_STAMINA_MINUTES, StaminaOutOfRange, regenerate_stamina_minutes,
};

#[test]
fn invalid_stamina_is_refused_even_when_no_gain_or_full_recovery_would_apply() {
    for stamina in [2_521, u32::MAX] {
        for elapsed in [0, 600, 780, MAX_OFFLINE_SECONDS, u64::MAX] {
            assert_eq!(
                regenerate_stamina_minutes(stamina, elapsed),
                Err(StaminaOutOfRange)
            );
        }
    }
}

#[test]
fn ten_minute_delay_precedes_three_minute_recovery_periods() {
    for (elapsed, gain) in [
        (0, 0),
        (599, 0),
        (600, 0),
        (601, 0),
        (779, 0),
        (780, 1),
        (781, 1),
        (959, 1),
        (960, 2),
    ] {
        assert_eq!(regenerate_stamina_minutes(1_000, elapsed), Ok(1_000 + gain));
    }
}

#[test]
fn crossing_thirty_nine_hours_spends_the_fast_period_before_the_slow_period() {
    for (elapsed, after) in [
        (779, 2_339),
        (780, 2_340),
        (959, 2_340),
        (960, 2_340),
        (1_139, 2_340),
        (1_140, 2_341),
        (1_499, 2_341),
        (1_500, 2_342),
    ] {
        assert_eq!(regenerate_stamina_minutes(2_339, elapsed), Ok(after));
    }
    // Two fast periods (360 s), then one slow period (360 s), after the delay.
    assert_eq!(regenerate_stamina_minutes(2_338, 1_319), Ok(2_340));
    assert_eq!(regenerate_stamina_minutes(2_338, 1_320), Ok(2_341));
}

#[test]
fn at_and_above_thirty_nine_hours_each_gain_needs_six_minutes_after_the_delay() {
    for stamina in [2_340, 2_341, 2_519, 2_520] {
        for elapsed in [600, 779, 780, 959] {
            assert_eq!(regenerate_stamina_minutes(stamina, elapsed), Ok(stamina));
        }
        assert_eq!(
            regenerate_stamina_minutes(stamina, 960),
            Ok((stamina + 1).min(2_520))
        );
    }
}

#[test]
fn empty_stamina_recovers_to_exactly_forty_two_hours_without_rounding_up() {
    assert_eq!(regenerate_stamina_minutes(0, 421_799), Ok(2_339));
    assert_eq!(regenerate_stamina_minutes(0, 421_800), Ok(2_340));
    assert_eq!(regenerate_stamina_minutes(0, 422_159), Ok(2_340));
    assert_eq!(regenerate_stamina_minutes(0, 422_160), Ok(2_341));
    assert_eq!(regenerate_stamina_minutes(0, 486_599), Ok(2_519));
    assert_eq!(regenerate_stamina_minutes(0, 486_600), Ok(2_520));
}

#[test]
fn duration_cap_and_maximum_integer_inputs_never_overflow_or_exceed_the_stamina_cap() {
    assert_eq!(MAX_OFFLINE_SECONDS, 1_814_400);
    assert_eq!(MAX_STAMINA_MINUTES, 2_520);
    // Recovery reaches 42 h before 21 days, so the cap's result is already saturated.
    // Cap-1/cap/cap+1 and maximum integer inputs must all preserve that bounded result.
    for stamina in [0, 1, 2_339, 2_340, 2_519, 2_520] {
        for elapsed in [
            MAX_OFFLINE_SECONDS - 1,
            MAX_OFFLINE_SECONDS,
            MAX_OFFLINE_SECONDS + 1,
            u64::MAX - 1,
            u64::MAX,
        ] {
            assert_eq!(regenerate_stamina_minutes(stamina, elapsed), Ok(2_520));
        }
    }
}

/// Independent period-by-period oracle: consumes time rather than using §5's closed formula.
fn period_oracle(mut stamina: u32, elapsed: u64) -> u32 {
    let mut remaining = elapsed.min(1_814_400).saturating_sub(600);
    while stamina < 2_520 {
        let period = if stamina < 2_340 { 180 } else { 360 };
        if remaining < period {
            break;
        }
        remaining -= period;
        stamina += 1;
    }
    stamina
}

#[test]
fn closed_formula_matches_an_independent_period_oracle_at_boundary_neighborhoods() {
    for stamina in [0, 1, 1_000, 2_338, 2_339, 2_340, 2_341, 2_519, 2_520] {
        for elapsed in [
            0,
            599,
            600,
            779,
            780,
            959,
            960,
            1_139,
            1_140,
            1_319,
            1_320,
            421_799,
            421_800,
            422_159,
            422_160,
            486_599,
            486_600,
            1_814_399,
            1_814_400,
            1_814_401,
            u64::MAX,
        ] {
            assert_eq!(
                regenerate_stamina_minutes(stamina, elapsed),
                Ok(period_oracle(stamina, elapsed)),
                "starting stamina {stamina}, raw seconds {elapsed}"
            );
        }
    }
}

#[test]
fn every_valid_start_is_bounded_and_monotonic_over_increasing_raw_durations() {
    for stamina in 0..=2_520 {
        let mut previous = stamina;
        for elapsed in [0, 600, 780, 960, 421_800, 486_600, 1_814_400, u64::MAX] {
            let result = regenerate_stamina_minutes(stamina, elapsed);
            assert!(matches!(result, Ok(value) if (previous..=2_520).contains(&value)));
            if let Ok(value) = result {
                previous = value;
            }
        }
    }
}
