//! Native lethal settlement and real dispatch into backpack and nested custody.
use super::TestResult;
use super::boss_rewards_fixtures::{Case, run_case};

#[test]
fn boss_personal_reward_collect_mints_child_inside_parent_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(Case::NestedCustody)
}
