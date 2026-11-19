//! Crystal 201189f8 action-3-third_seal_lever.lua:3-45 and movement-3-third_seal_flame.lua:3-31.

use super::TestResult;

#[test]
fn demonrage_uses_source_cues_and_resets_shared_switches_for_the_flame_entrant() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route_fixtures::demonrage_switches(admin)
}
