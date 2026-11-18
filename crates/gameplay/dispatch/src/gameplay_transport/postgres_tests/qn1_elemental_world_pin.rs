//! Exercise the real ambient owner against the retained Elemental Spheres fire map.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn qn1_elemental_world_pass_accepts_native_bundle_pin() -> TestResult {
    run("qn1-elemental-world-pin")
}

#[test]
fn qn1_elemental_live_drop_consumes_corpse_and_projects_native_boss() -> TestResult {
    run("qn1-elemental-arrival")
}

#[test]
fn qn1_elemental_soil_use_consumes_one_replays_and_recovers_decay() -> TestResult {
    run("qn1-elemental-soil")
}

#[test]
fn qn1_elemental_energy_soil_creates_field_without_replacing_altar() -> TestResult {
    run("qn1-elemental-energy-soil")
}

#[test]
fn qn1_elemental_energy_field_retires_once_through_world_reset() -> TestResult {
    run("qn1-elemental-energy-reset")
}

#[test]
fn elemental_energy_soil_tile_cap_returns_not_possible() -> TestResult {
    run("qn1-elemental-energy-cap")
}

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks_fixtures::scenario(
                name,
                if name.starts_with("qn1-elemental-energy-") {
                    Position {
                        x: 33094,
                        y: 32189,
                        floor: -13,
                    }
                } else {
                    Position {
                        x: 33199,
                        y: 32103,
                        floor: -13,
                    }
                },
                false,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "elemental World pass scenario panicked")??;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "QN1_ELEMENTAL_WORLD_PIN_PG_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
