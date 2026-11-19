//! Selected source-native tower puzzles; each probe grows into its physical route proof.

use super::TestResult;

fn scenario(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED {name}");
    super::qengine_rewards_fixtures::scenario(name)
}
#[test]
fn l6_paradox_native_first_grass_stair() -> TestResult {
    scenario("qengine-rewards-questpc4-native-tower-first")
}
#[test]
fn l6_paradox_native_second_exact_levers_ladder() -> TestResult {
    scenario("qengine-rewards-questpc4-native-tower-second")
}
#[test]
fn l6_paradox_native_third_ghoul_switch_and_plate() -> TestResult {
    scenario("qengine-rewards-questpc4-native-tower-third")
}
