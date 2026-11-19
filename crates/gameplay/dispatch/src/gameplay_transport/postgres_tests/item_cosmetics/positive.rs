use super::*;
use super::super::item_cosmetics_fixtures::positive::*;

#[test]
fn item_cosmetics_competing_source_grant_prevents_fourth_item_spend() -> TestResult {
    run_all(false, false, Some(Case::ConcurrentGrant))
}

#[test]
fn item_cosmetics_parts_replay_duplicate_and_appearance_consumer() -> TestResult {
    run_all(false, false, Some(Case::Parts))
}

#[test]
fn item_cosmetics_omitted_appearance_rolls_back_contribution_and_source() -> TestResult {
    run_all(false, false, Some(Case::OmittedGrant))
}

#[test]
fn item_cosmetics_outfit_addons_prerequisite_duplicate_and_full_achievement() -> TestResult {
    run_all(false, false, Some(Case::Outfit))
}

#[test]
fn item_cosmetics_ground_afflicted_commits_six_nested_pieces() -> TestResult {
    run_all(true, true, Some(Case::Afflicted))
}
