//! Crystal 201189f8 register_actions.lua:698-750, Breaking the Ice.
use super::TestResult;
use super::qcx60_chakoya_fixtures::{CRACK, NAME};

#[test]
fn picking_the_ice_spawns_chakoyas_and_preserves_the_existing_crack_reset() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || {
        super::quest_world_callbacks_fixtures::scenario(NAME, CRACK, false)
    })
}
