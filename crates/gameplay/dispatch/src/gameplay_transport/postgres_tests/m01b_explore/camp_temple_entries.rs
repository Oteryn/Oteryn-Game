use super::*;
use super::super::m01b_explore_fixtures::camp_temple_entries::*;

#[test]
fn m01b_camp_temple_iron_helmet_native_camp_entry_credits_once_and_replays() -> TestResult {
    native_case(
        "m01b-iron-helmet-camp",
        "m01b/iron-helmet/s1/ironhelmet",
        32770,
        32225,
        -7,
    )
}

#[test]
fn m01b_camp_temple_goblin_native_temple_entry_credits_once_and_replays() -> TestResult {
    native_case(
        "m01b-goblin-temple",
        "m01b/goblin-temple/s1/goblintemple",
        31973,
        32210,
        -12,
    )
}
