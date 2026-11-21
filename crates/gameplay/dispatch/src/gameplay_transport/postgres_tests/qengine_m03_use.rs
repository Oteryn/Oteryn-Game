//! The authored anniversary entrance and free return through ordinary network USE.
use super::qengine_m03_use_fixtures::{ENTRY, NAME};

use super::TestResult;


#[test]
fn m03_time_machine_native_use_live_guards_replay_and_free_return_postgres() -> TestResult {
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, ENTRY, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
