//! A timed personal trial STEP passes its encounter preflight, commits and schedules.
use super::m01c_stampede_entry_fixtures::{NAME, SELECTOR};

use super::TestResult;

#[test]
fn timed_personal_stampede_entry_passes_preflight_and_schedules_elephants() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("m01c-stampede-entry", || {
        super::quest_world_callbacks_fixtures::scenario(NAME, SELECTOR, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01c_stampede_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
