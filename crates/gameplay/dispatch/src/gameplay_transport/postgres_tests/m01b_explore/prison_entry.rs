use super::*;
use super::super::m01b_explore_fixtures::prison_entry::*;

#[test]
fn m01b_prison_native_hall_entry_requires_durable_level_and_replays_once() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-prison-hall", || {
        super::super::quest_world_callbacks_fixtures::scenario(
            NAME,
            Position {
                x: 32396,
                y: 32138,
                floor: -13,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_prison_hall duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
