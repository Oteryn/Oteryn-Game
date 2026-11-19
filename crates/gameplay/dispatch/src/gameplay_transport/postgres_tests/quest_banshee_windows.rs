//! Crystal 201189f8 action-1-first_seal_lever.lua:3-45 and movement-1-first_seal_close_mw.lua:1-73.

use super::TestResult;

#[test]
fn banshee_first_seal_honors_independent_windows_and_the_trapdoor_floor_route() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::banshee_windows(admin)
}
