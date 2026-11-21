use super::*;

#[test]
fn m01b_grave_danger_twelve_native_visits_require_level_premium_and_cannot_be_farmed() -> TestResult
{
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-grave-danger-visits", || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/grave-danger/s1/dark-cathedral",
                Position {
                    x: 32645,
                    y: 32394,
                    floor: -8,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_grave_danger_visits duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
