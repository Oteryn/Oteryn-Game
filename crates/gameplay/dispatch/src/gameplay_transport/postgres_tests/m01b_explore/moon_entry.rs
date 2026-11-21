use super::*;
use super::super::m01b_explore_fixtures::moon_entry::*;

#[test]
fn m01b_grimvale_native_visit_requires_current_moon_premium_and_reset() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-grimvale-entry", || {
        super::super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 33335,
                y: 31690,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_grimvale_entry duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
