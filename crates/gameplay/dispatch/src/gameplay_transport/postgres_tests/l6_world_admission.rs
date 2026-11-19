//! Physical progress and repeatable exact floor inputs through the admitted PostgreSQL owner.

use super::TestResult;

fn scenario(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED {name}");
    super::qengine_rewards_fixtures::scenario(name)
}
#[test]
fn l6_world_admission_opening_only_charges_exact_inputs_and_closes_free() -> TestResult {
    scenario("qengine-rewards-questpc4-admission-opening")
}
#[test]
fn l6_world_admission_opening_fifth_food_pair_closes_free() -> TestResult {
    scenario("qengine-rewards-questpc4-admission-opening-five")
}
#[test]
fn l6_world_admission_physical_checkpoint_rejects_generic_writer() -> TestResult {
    scenario("qengine-rewards-questpc4-admission-physical")
}
