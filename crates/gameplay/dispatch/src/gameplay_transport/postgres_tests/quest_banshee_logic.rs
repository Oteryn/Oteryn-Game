//! Crystal 201189f8 action-6-sixth_seal_lever.lua:3-32 and movement-6-sixth_seal_flame.lua:3-35.

use super::TestResult;

#[test]
fn banshee_logic_flame_rejects_extra_lit_campfires_and_accepts_the_full_layout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::logic_campfire_layout(admin)
}
