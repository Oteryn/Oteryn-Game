use super::{
    TestResult,
    alawars_vault_journey_fixtures::{NAME, START},
};
use std::time::Instant;

#[test]
fn alawars_vault_full_journey() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let start = Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, START, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result
}
