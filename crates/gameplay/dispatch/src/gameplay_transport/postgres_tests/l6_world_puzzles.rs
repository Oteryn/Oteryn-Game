//! Generic world puzzle mechanisms through the admitted world/character harness.

use super::TestResult;

fn scenario(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED {name}");
    super::qengine_rewards_fixtures::scenario(name)
}
#[test]
fn l6_world_puzzle_readonly_requirements_guard_native_state() -> TestResult {
    scenario("qengine-rewards-questpc4-puzzle-predicate")
}
#[test]
fn l6_world_puzzle_readonly_requirements_refuse_wrong_state() -> TestResult {
    scenario("qengine-rewards-questpc4-puzzle-predicate-refuse")
}
#[test]
fn l6_world_puzzle_presence_is_owned_and_creature_effects_are_value_free() -> TestResult {
    scenario("qengine-rewards-questpc4-puzzle-presence")
}
#[test]
fn l6_world_puzzle_ladder_use_commits_sealed_ascent() -> TestResult {
    scenario("qengine-rewards-questpc4-puzzle-ladder")
}
