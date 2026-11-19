//! Crystal 201189f8 data-global/scripts/quests/targuna/creaturescripts_herald_of_fire.lua:4-7,51-59.

use super::TestResult;
use super::qewrath_herald_escape_fixtures::{NAME, PORTAL};

#[test]
fn herald_death_reveals_one_traversable_escape_portal() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, PORTAL, false)
    })
}
