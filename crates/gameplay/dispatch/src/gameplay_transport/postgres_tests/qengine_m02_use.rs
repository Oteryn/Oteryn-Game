//! Nomads window through ordinary USE and the committed native scene.
use super::qengine_m02_use_fixtures::{NAME, WINDOW};

use super::TestResult;


#[test]
fn m02_nomads_native_use_observes_both_live_stacks_postgres() -> TestResult {
    scenario(NAME)
}
#[test]
fn m02_nomads_native_use_refuses_same_counts_in_wrong_order_postgres() -> TestResult {
    scenario("m02/nomads-land/window-wrong-order")
}
fn scenario(name: &'static str) -> TestResult {
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(name, WINDOW, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
