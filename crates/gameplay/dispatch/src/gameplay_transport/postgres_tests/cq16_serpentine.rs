//! Use the Serpentine Tower lever through production dispatch.
use super::TestResult;
use super::serpentine_tower_fixtures::{LEVER, NAME};

#[test]
fn serpentine_lever_opens_and_restores_all_four_walls() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("serpentine-lever", || {
        super::quest_world_callbacks_fixtures::scenario(NAME, LEVER, false)
    })
}
