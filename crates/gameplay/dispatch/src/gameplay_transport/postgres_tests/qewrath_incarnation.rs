//! Crystal 201189f8 actions_mission10_a_message_of_freedom_sceptre.lua:16-19,

use super::TestResult;
use super::qewrath_incarnation_fixtures::{NAME, STATUE};

#[test]
fn sceptre_awakens_the_statue_and_only_its_exact_incarnation_death_restores_it() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, STATUE, false)
    })
}
