//! Ahmet's mission, the Darama oasis box, and the hand-in for 100 brown mushrooms and 100 gold.

use super::TestResult;

#[test]
fn qd_steal_from_thieves_mission_bag_return_and_global_reward() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let name = super::qd_world_fixtures::AHMET;
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(
            name,
            super::qd_world_fixtures::start(name)?,
            false,
        )
    })
}
