//! Generic locked NPC level admission, with the selected Paradox start as its first consumer.

use super::TestResult;

#[test]
fn l6_npc_minimum_level_renamed_stripped_binding_cannot_use_generic_writer() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED l6_npc_minimum_level_shadow");
    super::qengine_rewards_fixtures::scenario("qengine-rewards-questpc4-paradox-shadow")
}

#[test]
fn l6_npc_minimum_level_uses_locked_progression_and_bound_terms() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED l6_npc_minimum_level");
    super::qengine_rewards_fixtures::scenario("qengine-rewards-questpc4-paradox-level")
}
