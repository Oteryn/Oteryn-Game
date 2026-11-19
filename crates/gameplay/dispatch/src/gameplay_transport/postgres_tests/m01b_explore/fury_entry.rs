use super::*;

#[test]
fn m01b_fury_native_survey_requires_selected_city_and_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-fury-survey", || {
        super::super::quest_world_callbacks_fixtures::scenario(
            "m01b/fury-gates-mini-world-change/s1/entry",
            Position {
                x: 32264,
                y: 32165,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_fury_survey duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
