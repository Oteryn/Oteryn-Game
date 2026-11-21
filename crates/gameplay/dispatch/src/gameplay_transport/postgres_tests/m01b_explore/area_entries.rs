use super::*;
use super::super::m01b_explore_fixtures::area_entries::*;

#[test]
fn m01b_area_entries_dragon_tower_native_entry_credits_once_and_replays() -> TestResult {
    native_case(
        "m01b-dragon-tower-entry",
        "m01b/dragon-tower/s1/entry",
        33075,
        32169,
    )
}

#[test]
fn m01b_area_entries_berserk_potion_native_entry_requires_current_premium() -> TestResult {
    native_case(
        "m01b-berserk-potion-entry",
        "m01b/berserk-potion/s1/entry",
        32479,
        31179,
    )
}
