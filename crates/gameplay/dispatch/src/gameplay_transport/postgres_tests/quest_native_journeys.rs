//! Quest journeys using the native contacts and captured map cells.
use super::{
    TestResult, too_hot_research_fixtures as research, tower_defence_journey_fixtures as tower,
};

#[test]
fn too_hot_research_native_coffin_repairs_and_white_gem() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(research::NAME, || {
        super::quest_world_callbacks_fixtures::scenario(research::NAME, research::START, false)
    })
}

#[test]
fn tower_defence_native_contacts_earn_password_and_four_arrow_stacks() -> TestResult {
    super::qengine_rewards_fixtures::scenario(tower::NAME)
}
