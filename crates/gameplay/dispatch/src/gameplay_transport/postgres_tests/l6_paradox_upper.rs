//! Source-native upper food floors and selected protected climb arrival.
use super::TestResult;

fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario(name)
}
#[test]
fn l6_paradox_upper_native_four_foods() -> TestResult {
    run("qengine-rewards-questpc4-paradox-upper-four")
}
#[test]
fn l6_paradox_upper_native_five_foods() -> TestResult {
    run("qengine-rewards-questpc4-paradox-upper-five")
}
#[test]
fn l6_paradox_upper_native_complete_checkpoint() -> TestResult {
    run("qengine-rewards-questpc4-paradox-upper-complete")
}
