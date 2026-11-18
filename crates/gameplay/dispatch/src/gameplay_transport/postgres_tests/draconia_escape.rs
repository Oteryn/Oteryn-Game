//! Source escape uid35012: an offering and exact conditional flag reset share one receipt.
//! The NPC-acquired flag is seeded through the existing quest writer solely for this
//! composition fixture. This file does not qualify the NPC exchange or physical map placement.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn draconia_escape_missing_offering_recovers_without_progress_or_receipt() -> TestResult {
    scenario("draconia-escape-missing")
}
#[test]
fn draconia_escape_consumes_one_offering_and_retains_absent_flag() -> TestResult {
    scenario("draconia-escape-absent")
}
#[test]
fn draconia_escape_consumes_offering_and_resets_exact_flag_atomically() -> TestResult {
    scenario("draconia-escape-exact")
}
#[test]
fn draconia_escape_consumes_offering_without_rewinding_later_flag() -> TestResult {
    scenario("draconia-escape-later")
}

fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        // Fixture approach tile, not evidence of the donor map's uid placement.
        super::quest_world_callbacks_fixtures::scenario(
            name,
            Position {
                x: 32818,
                y: 31598,
                floor: -9,
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
