use super::*;

#[test]
fn m01b_deeplings_native_survey_requires_current_counter_world_and_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-deeplings-survey", || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/deeplings-worldchange/s1/entry",
                Position {
                    x: 33641,
                    y: 31379,
                    floor: -9,
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_deeplings_survey duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
