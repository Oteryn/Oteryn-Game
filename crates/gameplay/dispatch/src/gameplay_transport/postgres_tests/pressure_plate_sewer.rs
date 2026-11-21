//! Crystal movements_sewer.lua:1-45: uid25030 creates/removes one sewer435.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn devil_helmet_pressure_plate_creates_sewer_then_removes_it_on_exit_and_replay() -> TestResult {
    scenario("devil-helmet-sewer")
}

#[test]
fn devil_helmet_pressure_plate_preserves_existing_sewer_until_exit() -> TestResult {
    scenario("devil-helmet-sewer-existing")
}

#[test]
fn devil_helmet_pressure_plate_ignores_unrelated_unique_id() -> TestResult {
    scenario("devil-helmet-sewer-unrelated")
}

#[test]
fn devil_helmet_pressure_plate_created_sewer_accepts_native_use_and_replay() -> TestResult {
    scenario("devil-helmet-sewer-use")
}

#[test]
fn devil_helmet_pressure_plate_existing_sewer_accepts_native_use_and_replay() -> TestResult {
    scenario("devil-helmet-sewer-use-existing")
}

#[test]
fn devil_helmet_pressure_plate_sewer_refuses_missing_landing() -> TestResult {
    scenario("devil-helmet-sewer-use-blocked")
}

fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("devil-helmet-sewer", move || {
        super::quest_world_callbacks_fixtures::scenario(
            name,
            Position {
                x: if name.contains("-use") { 32481 } else { 32480 },
                y: 32170,
                floor: -14,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
