//! Find Fiend through admitted spell speech and the live Forge population.
use super::{TestResult, parameter_spell_fixtures::scenario};

#[test]
fn find_fiend_locates_live_forge_creature_and_charges_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(7)
}

#[test]
fn find_fiend_refuses_without_a_live_fiend_and_does_not_charge() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(8)
}
