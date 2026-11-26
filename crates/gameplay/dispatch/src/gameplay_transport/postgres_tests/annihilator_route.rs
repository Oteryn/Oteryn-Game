//! Continuous source-region Annihilator route with genuine Fresh participants and D88 levels.
use super::annihilator_route_fixtures::{Route, run};

use super::TestResult;

#[test]
fn annihilator_lever_teleports_party_to_arrivals_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(Route::WireEntry)
}

#[test]
fn annihilator_source_route_genuine_party_entry() -> TestResult {
    run(Route::Entry)
}

#[test]
fn annihilator_source_route_native_combat_clearance() -> TestResult {
    run(Route::Combat)
}

#[test]
fn annihilator_source_route_earned_reward_exit_and_reentry() -> TestResult {
    run(Route::Complete)
}
