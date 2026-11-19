//! Crystal 201189f8, ferumbras_ascension/actions_flower_puzzle_lever.lua:1-11,29-37,63-66.
use super::TestResult;
use super::shulgrax_flower_pattern_fixtures::{CASE, CYCLE, LEVER, PORTAL, PORTAL_CASE, SWITCH};

#[test]
fn shulgrax_lever_requires_all_nine_generated_flowers_in_their_original_positions() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(CASE, || {
        super::quest_world_callbacks_fixtures::scenario(CASE, LEVER, false)
    })
}

#[test]
fn shulgrax_floor_switch_walk_cycles_through_three_flower_positions_and_clears() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(CYCLE, || {
        super::quest_world_callbacks_fixtures::scenario(CYCLE, SWITCH, false)
    })
}

#[test]
fn shulgrax_portal_admits_only_the_character_with_a_current_flower_flag() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(PORTAL_CASE, || {
        super::quest_world_callbacks_fixtures::scenario(PORTAL_CASE, PORTAL, false)
    })
}
