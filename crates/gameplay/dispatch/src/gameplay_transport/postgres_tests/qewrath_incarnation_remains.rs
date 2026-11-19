//! Crystal 201189f8 actions_mission10_a_message_of_freedom_sceptre.lua:8-9,20-32,

use super::TestResult;
use super::qewrath_incarnation_remains_fixtures::{BODY, NAME};

#[test]
fn banishing_native_incarnation_remains_credits_once_within_the_sceptre_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, BODY, false)
    })
}
