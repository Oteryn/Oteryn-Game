use super::*;
use super::super::m01b_explore_fixtures::hero_fire_entry::*;

#[test]
fn m01b_hero_fire_native_visit_requires_first_survey_evidence_and_premium() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result =
        super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-hero-fire-visit", || {
            super::super::quest_world_callbacks_fixtures::scenario(
                "m01b/hero-of-rathleton/s3/entry",
                Position {
                    x: START.x - 1,
                    ..START
                },
                false,
            )
        });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_hero_fire_visit duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
