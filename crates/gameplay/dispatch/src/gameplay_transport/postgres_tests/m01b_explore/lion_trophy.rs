use super::*;

#[test]
fn m01b_lion_trophy_requires_own_completed_ice_islands_and_current_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-lion-trophy", || {
        super::super::quest_world_callbacks_fixtures::scenario(
            "m01b/lion-trophy/s1/entry",
            Position {
                x: 32146,
                y: 31120,
                floor: -9,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_lion_trophy duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
