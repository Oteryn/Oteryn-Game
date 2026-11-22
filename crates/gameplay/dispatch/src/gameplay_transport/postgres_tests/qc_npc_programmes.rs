//! The Sweaty Cyclops's draconian steel exchange through native NPC dispatch.
use super::{TestResult, questpc7_friends_fixtures::run};

#[test]
fn qc_cyclops_steel_replaces_amulet_quote() -> TestResult {
    run("questpc7-friends-draconian-replaces-amulet")
}

#[test]
fn qc_cyclops_steel_is_atomic_and_repeatable() -> TestResult {
    run("questpc7-friends-draconian-repeat")
}

#[test]
fn qc_cyclops_steel_refuses_without_shield() -> TestResult {
    run("questpc7-friends-draconian-no-shield")
}

#[test]
fn questpc7_friends_resumes_saved_definition() -> TestResult {
    run(super::questpc7_friends_fixtures::LEGACY_CASE)
}
