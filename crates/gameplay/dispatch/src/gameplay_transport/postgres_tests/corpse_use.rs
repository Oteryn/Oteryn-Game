use super::TestResult;
use super::kill_reward_e2e_fixtures::{scenario_thread, scenario_with_corpse_use};

#[test]
fn rat_corpse_first_use_retries_busy_read_and_removed_corpse_is_stale() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario_thread::run("rat-corpse-use", scenario_with_corpse_use)
}
