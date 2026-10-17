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

#[test]
fn owned_visual_map_and_policy_contain_only_rain_fog_and_snow() -> Result<(), WeatherPolicyError> {
    assert_eq!(
        WEATHER_VISUAL_PRESETS,
        [
            (WeatherPreset(1), WeatherVisual::Rain),
            (WeatherPreset(2), WeatherVisual::Fog),
            (WeatherPreset(3), WeatherVisual::Snow),
        ]
    );
    let entries = [1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3];
    assert_eq!(
        WEATHER_VISUAL_POLICY,
        WeatherPolicy::new(&[1, 2, 3], &entries)?
    );
    for (bucket, expected) in entries.into_iter().enumerate() {
        let preset = WEATHER_VISUAL_POLICY.preset_for_draw((bucket as u64) << 60);
        assert_eq!(preset.index(), expected);
        assert!(
            WEATHER_VISUAL_PRESETS
                .iter()
                .any(|(known, _)| *known == preset)
        );
    }
    Ok(())
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
    for bucket in 0..16_u64 {
        let _visual = WEATHER_VISUAL_POLICY.preset_for_draw(bucket << 60);
        let _selected = WEATHER_VISUAL_POLICY.select(&root, occurrence)?;
        assert_eq!(source.reading(time), Some(before));
        assert_eq!(
            deterministic_decision_u64(&root, occurrence, "gameplay.fixture.v1", 0)?,
            gameplay_before
        );
    }
    Ok(())
}
