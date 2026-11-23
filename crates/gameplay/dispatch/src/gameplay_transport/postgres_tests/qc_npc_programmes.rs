//! The Sweaty Cyclops's draconian steel exchange through native NPC dispatch.
use super::{TestResult, questpc7_friends_fixtures::run};

#[test]
fn qc_cyclops_steel_replaces_amulet_quote() -> TestResult {
    run("questpc7-friends-draconian-replaces-amulet")
}

#[test]
fn qc_cyclops_amulet_replaces_steel_quote() -> TestResult {
    run("questpc7-friends-draconian-amulet-replaces-steel")
}

#[test]
fn qc_cyclops_other_services_clear_steel_quote() -> TestResult {
    run("questpc7-friends-draconian-other-services")
}

#[test]
fn qc_cyclops_steel_matches_donor_keyword_and_exact_yes() -> TestResult {
    run("questpc7-friends-draconian-keyword")
}

#[test]
fn qc_cyclops_unlocks_steel_through_donor_dialogue() -> TestResult {
    run("questpc7-friends-draconian-unlock")
}

#[test]
fn qc_cyclops_steel_rolls_back_when_mint_refuses() -> TestResult {
    run("questpc7-friends-draconian-mint-refusal")
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

#[test]
fn qc_cyclops_donor_exchanges_replace_steel_quote() -> TestResult {
    run("questpc7-friends-draconian-donor-exchanges")
}

#[test]
fn qc_cyclops_locked_soul_orb_leaves_amulet_quote() -> TestResult {
    run("questpc7-friends-draconian-locked-soul-orb")
}

#[test]
fn qc_cyclops_missing_ingredient_keeps_all_four_metal_quotes() -> TestResult {
    run("questpc7-friends-draconian-ingredient-retry")
}
