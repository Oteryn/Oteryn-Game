use super::{TestResult, kill_reward_e2e_fixtures};

#[test]
fn p04_goblin_kill_removal_and_exact_corpse_share_the_observer_delta() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let result = kill_reward_e2e_fixtures::scenario_thread::run(
        "p04-goblin-corpse-delta",
        kill_reward_e2e_fixtures::goblin_corpse_scenario,
    );
    println!(
        "POSTGRES_CASE_COMPLETED p04_goblin_corpse_delta success={}",
        result.is_ok()
    );
    result
}
