use super::*;
use super::super::m01b_explore_fixtures::iceberg_chamber::*;

#[test]
fn m01b_chakoya_chamber_native_visit_requires_first_survey_kills_and_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-chakoya-chamber", || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/chakoya-iceberg-mini-world-change/s3/entry",
                Position {
                    y: START.y - 1,
                    ..START
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_chakoya_chamber duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
