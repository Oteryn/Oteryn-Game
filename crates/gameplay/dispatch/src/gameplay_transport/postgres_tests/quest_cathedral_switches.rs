//! Crystal 201189f8 dark_cathedral_teleports.lua:1-55.

use super::TestResult;

#[test]
fn cathedral_portal_requires_all_six_switches_and_closes_when_the_first_expires() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::cathedral_switches(admin)
}
