use super::TestResult;
use super::new_frontier_journey_fixtures::{BAIT_RECOVERY, NAME, start};
use super::quest_world_callbacks_fixtures::scenario;

#[test]
fn new_frontier_full_settlement_journey_earns_rewards() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || scenario(NAME, start(), false))
}

#[test]
fn new_frontier_melfar_replaces_bait_consumed_at_the_southern_tree() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(BAIT_RECOVERY, || {
        scenario(BAIT_RECOVERY, start(), false)
    })
}
