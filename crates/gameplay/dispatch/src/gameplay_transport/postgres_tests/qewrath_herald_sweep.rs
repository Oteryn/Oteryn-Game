//! Crystal 201189f8 movements_herald_red_floors.lua:31–39,59–83.

use super::TestResult;
use super::qewrath_herald_sweep_fixtures::{FIELD_NAME, LAST_PLATE, NAME};

#[test]
fn herald_last_plate_resets_the_room_monsters_and_preserves_the_herald() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, LAST_PLATE, false)
    })
}

#[test]
fn herald_last_plate_clears_a_real_native_field_and_the_room_summon() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(FIELD_NAME, || {
        super::quest_world_callbacks_fixtures::scenario(FIELD_NAME, LAST_PLATE, false)
    })
}
