use super::*;

#[test]
fn m01b_bewitched_native_personal_cauldron_visit_does_not_credit_the_brew() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::super::kill_reward_e2e_fixtures::scenario_thread::run("m01b-cauldron-visit", || {
        super::super::quest_world_callbacks_fixtures::scenario(
            "m01b/bewitched/s1/entry",
            Position {
                x: 32733,
                y: 31935,
                floor: -12,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED m01b_cauldron_visit duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
