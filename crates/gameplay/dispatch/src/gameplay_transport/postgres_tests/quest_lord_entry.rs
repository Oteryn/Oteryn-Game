//! Crystal 201189f8 elemental_spheres/actions_lever.lua:1-73 and its shared BossRoom gate.

use super::TestResult;

#[test]
fn lord_chamber_entry_checks_each_member_and_refuses_during_the_window() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::lord_entry(admin)
}
