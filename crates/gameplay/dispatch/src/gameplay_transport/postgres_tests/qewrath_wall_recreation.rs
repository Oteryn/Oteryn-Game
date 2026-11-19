//! Crystal 201189f8 actions_mission01_lights.lua:8–12,17–29,47–68.

use super::TestResult;
use super::qewrath_wall_recreation_fixtures::{LAMP_LEVER, LAMP_NAME, LEVER, NAME};

#[test]
fn shrine_lever_flips_while_the_wall_passage_keeps_its_original_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, LEVER, false)
    })
}

#[test]
fn shrine_lamp_timer_transforms_and_restores_only_the_matching_lamp() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(LAMP_NAME, || {
        super::quest_world_callbacks_fixtures::scenario(LAMP_NAME, LAMP_LEVER, false)
    })
}
