//! Crystal 201189f8, forgotten_knowledge/movements_entrance_teleport.lua:12-20,100-119.
use super::TestResult;
use super::forgotten_temple_portal_fixtures::{CASE, PORTAL};

#[test]
fn forgotten_temple_energy_portal_cues_both_ends_and_refuses_unqualified_peer() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(CASE, || {
        super::quest_world_callbacks_fixtures::scenario(CASE, PORTAL, false)
    })
}
