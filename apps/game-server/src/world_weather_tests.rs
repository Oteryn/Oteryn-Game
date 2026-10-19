#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use serde_json::{Value, json};

const INDEX: &str = include_str!("../../../content/weather/index.json");
const CLIMATES: &str = include_str!("../../../content/weather/climates.json");
const AREAS: &str = include_str!("../../../content/weather/areas.json");

fn load() -> WorldWeatherSource {
    WorldWeatherSource::from_json(INDEX, CLIMATES, AREAS).expect("shipped data loads")
}

fn edited(text: &str, edit: impl FnOnce(&mut Value)) -> String {
    let mut value: Value = serde_json::from_str(text).expect("json");
    edit(&mut value);
    value.to_string()
}

fn at(x: i32, y: i32) -> ActorPosition {
    ActorPosition { x, y, floor: 7 }
}

fn active(weather: WorldWeather) -> (u64, u8, u8) {
    match weather {
        WorldWeather::Active {
            absolute_slot,
            kind,
            intensity,
            ..
        } => (absolute_slot, kind, intensity),
        WorldWeather::Unavailable => panic!("expected an active observation"),
    }
}

fn area<'a>(source: &'a WorldWeatherSource, key: &str) -> &'a WeatherArea {
    source
        .areas()
        .iter()
        .find(|candidate| candidate.key == key)
        .expect("area exists")
}

#[test]
fn the_shipped_data_loads_and_is_the_production_source() {
    let source = load();
    assert_eq!(source.areas().len(), 50);
    assert!(shipped().is_some());
}

#[test]
fn periods_are_aligned_one_to_three_slots_and_respect_the_climate() {
    let source = load();
    for area in source.areas() {
        let mut previous: Option<WeatherPeriod> = None;
        let mut run = 0_u64;
        for slot in 0..(BLOCK_SLOTS * 40) {
            let period = source.source().period(area, slot).expect("period");
            assert!(period.start_slot <= slot && slot - period.start_slot < 3);
            assert!((1..=WEATHER_KIND_MAX).contains(&period.kind));
            assert!((1..=WEATHER_INTENSITY_MAX).contains(&period.intensity));
            if slot % BLOCK_SLOTS == 0 {
                assert_eq!(period.start_slot, slot);
            }
            if previous.is_some_and(|prior| prior.start_slot == period.start_slot) {
                run += 1;
                assert_eq!(previous.map(|prior| prior.kind), Some(period.kind));
                assert_eq!(
                    previous.map(|prior| prior.intensity),
                    Some(period.intensity)
                );
                assert!(run < 3);
            } else {
                run = 0;
            }
            previous = Some(period);
        }
    }
    // Cold areas never rain; desert areas never snow.
    let cold = area(&source, "oteryn:content.area.city.svargrond");
    let desert = area(&source, "oteryn:content.area.city.ankrahmun");
    for slot in 0..5_000 {
        let kind = source.source().period(cold, slot).expect("period").kind;
        assert!([1, 6, 7].contains(&kind), "cold kind {kind}");
        let kind = source.source().period(desert, slot).expect("period").kind;
        assert!([1, 8, 9].contains(&kind), "desert kind {kind}");
    }
}

#[test]
fn weather_is_a_pure_function_of_area_and_slot() {
    let first = load();
    let second = load();
    for slot in 0..500 {
        let now = slot * WEATHER_SLOT_MS + 17;
        assert_eq!(
            first.observe(at(32369, 32241), now),
            second.observe(at(32369, 32241), now)
        );
    }
}

#[test]
fn areas_draw_independently() {
    let source = load();
    let a = area(&source, "oteryn:content.area.city.thais");
    let b = area(&source, "oteryn:content.area.city.carlin");
    let differing = (0..2_000)
        .filter(|slot| source.source().period(a, *slot) != source.source().period(b, *slot))
        .count();
    assert!(differing > 400, "areas look correlated: {differing}");
}

#[test]
fn kind_frequencies_follow_the_temperate_weights() {
    let source = load();
    let thais = area(&source, "oteryn:content.area.city.thais");
    let mut counts = [0_u32; 10];
    let blocks = 20_000_u64;
    for block in 0..blocks {
        counts[usize::from(
            source
                .source()
                .period(thais, block * BLOCK_SLOTS)
                .expect("period")
                .kind,
        )] += 1;
    }
    let share = |kind: usize| f64::from(counts[kind]) / blocks as f64;
    assert!((share(1) - 0.55).abs() < 0.03, "clear {}", share(1));
    assert!((share(2) - 0.20).abs() < 0.03, "cloudy {}", share(2));
    assert!((share(3) - 0.15).abs() < 0.03, "rain {}", share(3));
    assert!((share(4) - 0.07).abs() < 0.02, "storm {}", share(4));
    assert!((share(5) - 0.03).abs() < 0.02, "fog {}", share(5));
}

#[test]
fn intensity_is_uniform_one_to_three() {
    let source = load();
    let thais = area(&source, "oteryn:content.area.city.thais");
    let mut counts = [0_u32; 4];
    let blocks = 9_000_u64;
    for block in 0..blocks {
        counts[usize::from(
            source
                .source()
                .period(thais, block * BLOCK_SLOTS)
                .expect("period")
                .intensity,
        )] += 1;
    }
    assert_eq!(counts[0], 0);
    for (intensity, count) in counts.iter().enumerate().skip(1) {
        let share = f64::from(*count) / blocks as f64;
        assert!((share - 1.0 / 3.0).abs() < 0.03, "intensity {intensity}");
    }
}

#[test]
fn a_position_maps_to_the_nearest_anchor_and_observes_that_area() {
    let source = load();
    let thais = area(&source, "oteryn:content.area.city.thais");
    let anchor = thais.anchor.expect("thais is anchored");
    let found = source
        .area_at(at(anchor.0, anchor.1))
        .expect("an area resolves");
    assert_eq!(found.key, thais.key);
    let now = 7 * WEATHER_SLOT_MS;
    let (start, kind, intensity) = active(source.observe(at(anchor.0, anchor.1), now));
    let expected = source.source().period(thais, 7).expect("period");
    assert_eq!(
        (start, kind, intensity),
        (expected.start_slot, expected.kind, expected.intensity)
    );
    // Far from everything still resolves to some anchored area (never a fictitious default).
    assert!(source.area_at(at(i32::MAX, i32::MIN)).is_some());
}

#[test]
fn the_observation_changes_with_the_period_and_not_inside_it() {
    let source = load();
    let position = at(32369, 32241);
    let mut seen = std::collections::BTreeSet::new();
    let mut last = None;
    for slot in 0..300_u64 {
        let observed = source.observe(position, slot * WEATHER_SLOT_MS + 5);
        let (start, _, _) = active(observed);
        if last == Some(start) {
            // Inside one period the observation is identical to its first slot.
            assert_eq!(observed, source.observe(position, start * WEATHER_SLOT_MS));
        }
        seen.insert(start);
        last = Some(start);
    }
    assert!(seen.len() > 100 && seen.len() < 300);
}

#[test]
fn invalid_or_missing_data_fails_closed() {
    assert!(WorldWeatherSource::from_json("", CLIMATES, AREAS).is_err());
    assert!(WorldWeatherSource::from_json(INDEX, "{", AREAS).is_err());
    assert!(WorldWeatherSource::from_json(INDEX, CLIMATES, "[]").is_err());
    let bad_index = edited(INDEX, |v| v["schema"] = json!("x"));
    assert!(WorldWeatherSource::from_json(&bad_index, CLIMATES, AREAS).is_err());
    let bad_slot = edited(INDEX, |v| v["slot_ms"] = json!(1000));
    assert!(WorldWeatherSource::from_json(&bad_slot, CLIMATES, AREAS).is_err());
    let not_design = edited(INDEX, |v| v["design_status"] = json!("REFERENCE"));
    assert!(WorldWeatherSource::from_json(&not_design, CLIMATES, AREAS).is_err());
    let unknown_kind = edited(CLIMATES, |v| v["climates"]["cold"]["hail"] = json!(5));
    assert!(WorldWeatherSource::from_json(INDEX, &unknown_kind, AREAS).is_err());
    let zero_weight = edited(CLIMATES, |v| v["climates"]["cold"]["snow"] = json!(0));
    assert!(WorldWeatherSource::from_json(INDEX, &zero_weight, AREAS).is_err());
    let long = edited(CLIMATES, |v| v["duration_slots"]["max"] = json!(13));
    assert!(WorldWeatherSource::from_json(INDEX, &long, AREAS).is_err());
    let zero_duration = edited(CLIMATES, |v| v["duration_slots"]["min"] = json!(0));
    assert!(WorldWeatherSource::from_json(INDEX, &zero_duration, AREAS).is_err());
    let strong = edited(CLIMATES, |v| v["intensity"]["max"] = json!(4));
    assert!(WorldWeatherSource::from_json(INDEX, &strong, AREAS).is_err());
    let unknown_climate = edited(AREAS, |v| v["areas"][0]["climate"] = json!("lava"));
    assert!(WorldWeatherSource::from_json(INDEX, CLIMATES, &unknown_climate).is_err());
    let duplicate = edited(AREAS, |v| {
        let first = v["areas"][0].clone();
        v["areas"].as_array_mut().expect("array").push(first);
    });
    assert!(WorldWeatherSource::from_json(INDEX, CLIMATES, &duplicate).is_err());
    let no_anchor = edited(AREAS, |v| {
        for entry in v["areas"].as_array_mut().expect("array") {
            entry["anchor"] = Value::Null;
        }
    });
    assert!(WorldWeatherSource::from_json(INDEX, CLIMATES, &no_anchor).is_err());
    let extra = edited(AREAS, |v| v["areas"][0]["surprise"] = json!(1));
    assert!(WorldWeatherSource::from_json(INDEX, CLIMATES, &extra).is_err());
}

#[test]
fn real_world_coordinates_are_the_reserved_extension_point() {
    let with_point = edited(AREAS, |v| {
        v["areas"][0]["real_world"] = json!({"latitude": 52.2, "longitude": 21.0});
    });
    let source = WorldWeatherSource::from_json(INDEX, CLIMATES, &with_point).expect("valid");
    assert_eq!(source.areas()[0].real_world, Some((52.2, 21.0)));
    // Carrying coordinates changes nothing today: the design source ignores them.
    assert_eq!(
        source.observe(at(32369, 32241), 123_456_789),
        load().observe(at(32369, 32241), 123_456_789)
    );
    let out_of_range = edited(AREAS, |v| {
        v["areas"][0]["real_world"] = json!({"latitude": 91.0, "longitude": 0.0});
    });
    assert!(WorldWeatherSource::from_json(INDEX, CLIMATES, &out_of_range).is_err());
}
