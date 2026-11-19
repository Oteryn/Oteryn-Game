//! Crystal 201189f8 actions_mission03_the_keeper.lua:19–23.

use super::TestResult;
use super::qewrath_keeper_tail_fixtures::{BODY, BROWSE_NAME, NAME};

#[test]
fn keeper_corpse_plain_use_gives_the_tail_and_advances_mission_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, BODY, false)
    })
}

#[test]
fn keeper_corpse_browse_field_use_gives_the_same_tail_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(BROWSE_NAME, || {
        super::quest_world_callbacks_fixtures::scenario(BROWSE_NAME, BODY, false)
    })
}
