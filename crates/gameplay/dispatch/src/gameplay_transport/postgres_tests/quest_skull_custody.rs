use super::TestResult;
use super::quest_skull_custody_fixtures::{FIRST_VIEW, NAME};

#[test]
fn quest_bone_pile_mints_exact_skull_and_step_requires_its_carried_custody() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, FIRST_VIEW, false)
    })
}
