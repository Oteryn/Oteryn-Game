#![allow(clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn slots_use_absolute_elapsed_and_checked_subtraction() {
    let epoch = 18_000_000;
    for (elapsed, slot) in [
        (0, 0),
        (WEATHER_SLOT_MS - 1, 0),
        (WEATHER_SLOT_MS, 1),
        (WEATHER_SLOT_MS + 1, 1),
        (3_600_000, 4), // Tibian midnight must not reset weather's slot.
        (WEATHER_SLOT_MS * 1_000_000, 1_000_000),
    ] {
        assert_eq!(absolute_weather_slot(epoch, epoch + elapsed), Some(slot));
    }
    assert_eq!(absolute_weather_slot(epoch, epoch - 1), None);
    assert_eq!(absolute_weather_slot(u64::MAX, u64::MAX), Some(0));
    assert_eq!(
        absolute_weather_slot(1, u64::MAX),
        Some((u64::MAX - 1) / WEATHER_SLOT_MS)
    );
}

#[test]
fn policy_requires_an_explicit_bounded_admission_and_selector() {
    let all: Vec<u8> = (1..=16).collect();
    assert!(WeatherPolicy::new(&all, &all).is_ok());
    assert!(WeatherPolicy::new(&[16], &[16; 16]).is_ok());
    for (admitted, selector, expected) in [
        (vec![], vec![1; 16], WeatherPolicyError::PresetCount),
        (vec![1; 17], vec![1; 16], WeatherPolicyError::PresetCount),
        (vec![0], vec![1; 16], WeatherPolicyError::InvalidPreset),
        (vec![17], vec![1; 16], WeatherPolicyError::InvalidPreset),
        (vec![1, 1], vec![1; 16], WeatherPolicyError::DuplicatePreset),
        (vec![1], vec![1; 15], WeatherPolicyError::SelectorLength),
        (vec![1], vec![1; 17], WeatherPolicyError::SelectorLength),
        (vec![1], vec![0; 16], WeatherPolicyError::InvalidPreset),
        (vec![1], vec![17; 16], WeatherPolicyError::InvalidPreset),
        (vec![1], vec![2; 16], WeatherPolicyError::UnadmittedPreset),
    ] {
        assert_eq!(WeatherPolicy::new(&admitted, &selector), Err(expected));
    }
}

#[test]
fn every_draw_bucket_has_exact_boundaries_and_explicit_weights() -> Result<(), WeatherPolicyError> {
    // Fixture weights only; this is not a shipped rain/fog/snow policy.
    let entries = [1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3];
    let policy = WeatherPolicy::new(&[1, 2, 3], &entries)?;
    for (bucket, expected) in entries.into_iter().enumerate() {
        let low = (bucket as u64) << 60;
        let high = low | ((1_u64 << 60) - 1);
        assert_eq!(policy.preset_for_draw(low).index(), expected);
        assert_eq!(policy.preset_for_draw(high).index(), expected);
    }
    Ok(())
}

#[test]
fn selection_reuses_the_existing_sim_primitive_without_rng_state()
-> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<u8> = (1..=16).collect();
    let policy = WeatherPolicy::new(&entries, &entries)
        .map_err(|error| format!("fixture policy: {error:?}"))?;
    // Fixture bytes exercise the primitive only; they are not a production binding or the
    // canonical World/clock/policy/slot occurrence derivation (owned by WEATHER-SEAM-1).
    let root = GameplayDecisionRoot::from_bytes([0x31; 32]);
    let occurrence = DecisionOccurrenceId::from_bytes([0x72; 16]);
    let first = policy.select(&root, occurrence)?;
    assert_eq!(
        deterministic_decision_u64(&root, occurrence, WEATHER_PURPOSE, 0)?,
        0x4081_aa16_50ad_23d9
    );
    assert_eq!(first.index(), 5);
    for unrelated_draw in 0..32 {
        let _ = deterministic_decision_u64(&root, occurrence, "other.visual.v1", unrelated_draw)?;
        // Fresh model construction represents a retry/restart; no Channel is an input.
        let restarted = WeatherPolicy::new(&entries, &entries)
            .map_err(|error| format!("fixture policy: {error:?}"))?;
        assert_eq!(restarted.select(&root, occurrence)?, first);
    }
    for (root_byte, occurrence_byte, golden_draw, preset) in [
        (0x32, 0x72, 0x105d_272c_dd06_26aa, 2),
        (0x31, 0x73, 0x3927_5574_1e88_25eb, 4),
    ] {
        let changed_root = GameplayDecisionRoot::from_bytes([root_byte; 32]);
        let changed_occurrence = DecisionOccurrenceId::from_bytes([occurrence_byte; 16]);
        assert_eq!(
            deterministic_decision_u64(&changed_root, changed_occurrence, WEATHER_PURPOSE, 0)?,
            golden_draw
        );
        assert_eq!(
            policy.select(&changed_root, changed_occurrence)?.index(),
            preset
        );
    }
    Ok(())
}

fn test_world(last: u8) -> WorldId {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = last;
    WorldId::decode(&bytes).expect("fixture World id")
}

/// Fixture policy only (CP D1446: no shipped table exists); it exercises the seam.
fn test_source(seed: u8, world: u8, bound: u8) -> WorldWeatherSource {
    let all: Vec<u8> = (1..=16).collect();
    WorldWeatherSource::new(
        test_world(world),
        RetainedWorldConfig::for_tests(
            test_world(bound),
            [seed; 32],
            "clock-v1",
            18_000_000,
            "policy-7",
        ),
        WeatherPolicy::new(&all, &all).expect("fixture policy"),
    )
}

fn active(weather: WorldWeather) -> (u64, u8, [u8; 32]) {
    match weather {
        WorldWeather::Active {
            binding_token,
            absolute_slot,
            preset_index,
        } => (absolute_slot, preset_index, binding_token),
        WorldWeather::Unavailable => panic!("expected Active"),
    }
}

#[test]
fn source_observation_is_direct_slot_stable_across_restart_and_seedless_in_the_token() {
    let epoch = 18_000_000;
    let first = test_source(0x31, 1, 1);
    let (slot0, preset0, token) = active(first.observe(epoch));
    assert_eq!(slot0, 0);
    // P-1 stays in slot 0, P and P+1 are slot 1; an instant replacement, no history.
    assert_eq!(
        first.observe(epoch + WEATHER_SLOT_MS - 1),
        first.observe(epoch)
    );
    let (slot1, _, token1) = active(first.observe(epoch + WEATHER_SLOT_MS));
    assert_eq!(slot1, 1);
    assert_eq!(token1, token);
    assert_eq!(
        first.observe(epoch + WEATHER_SLOT_MS),
        first.observe(epoch + WEATHER_SLOT_MS + 1)
    );
    // A restarted process (fresh source from the same retained inputs) never rerolls, and the
    // order of earlier observations does not matter.
    let restarted = test_source(0x31, 1, 1);
    for offset in [
        5 * WEATHER_SLOT_MS,
        0,
        1_000_000 * WEATHER_SLOT_MS,
        WEATHER_SLOT_MS,
    ] {
        assert_eq!(
            restarted.observe(epoch + offset),
            first.observe(epoch + offset)
        );
    }
    assert_eq!(active(restarted.observe(epoch)).1, preset0);
    // A different seed changes the draw stream but not the public token; the token is not seed.
    let other_seed = test_source(0x32, 1, 1);
    assert_eq!(active(other_seed.observe(epoch)).2, token);
    assert!((0..64).any(|n| {
        active(other_seed.observe(epoch + n * WEATHER_SLOT_MS)).1
            != active(first.observe(epoch + n * WEATHER_SLOT_MS)).1
    }));
    assert!(!token.windows(32).any(|w| w == [0x31; 32]));
    // Another World's binding yields a different token.
    assert_ne!(active(test_source(0x31, 2, 2).observe(epoch)).2, token);
}

#[test]
fn source_fails_closed_without_a_readable_binding_or_before_the_epoch() {
    // The source's World differs from the retained config's World: not readable.
    assert_eq!(
        test_source(0x31, 1, 2).observe(18_000_000),
        WorldWeather::Unavailable
    );
    assert_eq!(
        test_source(0x31, 1, 1).observe(17_999_999),
        WorldWeather::Unavailable
    );
}

#[test]
fn visual_selection_does_not_mutate_native_clock_light_or_gameplay_decision()
-> Result<(), Box<dyn std::error::Error>> {
    // This exercises the actual existing clock and SIM primitive in their native unit context.
    // Transport, movement/combat/storage consumers are not present here; their isolation proof
    // remains a real-caller integration gate, not a claim based on a fabricated gameplay struct.
    let epoch = 18_000_000;
    let time = epoch + 1_234_567;
    let source = crate::world_clock::WorldClockSource::new(epoch);
    let before = source.reading(time).ok_or("fixture clock before epoch")?;
    let root = GameplayDecisionRoot::from_bytes([0x31; 32]);
    let occurrence = DecisionOccurrenceId::from_bytes([0x72; 16]);
    let gameplay_before = deterministic_decision_u64(&root, occurrence, "gameplay.fixture.v1", 0)?;
    // Fixture policy only (CP D1446: no shipped table exists).
    let all: Vec<u8> = (1..=16).collect();
    let policy = WeatherPolicy::new(&all, &all).map_err(|error| format!("{error:?}"))?;
    for bucket in 0..16_u64 {
        let _visual = policy.preset_for_draw(bucket << 60);
        let _selected = policy.select(&root, occurrence)?;
        assert_eq!(source.reading(time), Some(before));
        assert_eq!(
            deterministic_decision_u64(&root, occurrence, "gameplay.fixture.v1", 0)?,
            gameplay_before
        );
    }
    Ok(())
}
