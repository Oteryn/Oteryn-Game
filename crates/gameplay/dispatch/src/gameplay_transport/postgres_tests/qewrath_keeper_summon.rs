//! Crystal 201189f8 actions_mission03_the_keeper.lua:10–16.

use super::TestResult;
use super::qewrath_keeper_summon_fixtures::{NAME, PLANT};

#[test]
fn keeper_terminal_plant_use_consumes_the_selected_poison_and_summons_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, PLANT, false)
    })
}
