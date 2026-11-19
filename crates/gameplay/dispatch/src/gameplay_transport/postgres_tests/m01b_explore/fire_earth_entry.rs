use super::*;

#[test]
fn m01b_fire_earth_native_survey_requires_current_eruption_and_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-fire-earth-survey", || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/fire-from-the-earth-mini-world-change/s1/entry",
                Position {
                    x: 32072,
                    y: 32604,
                    floor: -2,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_fire_earth_survey duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
