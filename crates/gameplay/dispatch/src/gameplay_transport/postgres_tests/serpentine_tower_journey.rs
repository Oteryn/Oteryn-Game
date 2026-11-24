use super::TestResult;

#[test]
fn serpentine_tower_full_journey() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(
        "serpentine-tower",
        super::serpentine_tower_journey_fixtures::journey,
    );
    println!(
        "POSTGRES_CASE_COMPLETED serpentine_tower_full_journey duration={:?} success={}",
        started.elapsed(),
        result.is_ok(),
    );
    result
}
