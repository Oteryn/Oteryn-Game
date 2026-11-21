use super::*;
use super::super::m01b_explore_fixtures::treasure_entries::*;

#[test]
fn m01b_treasure_fishing_box_native_entry_requires_current_premium() -> TestResult {
    native_case(
        "m01b-fishing-box-entry",
        "m01b/fishing-box/s1/fishing_box",
        32016,
        31409,
        -7,
    )
}

#[test]
fn m01b_treasure_berserker_native_entry_requires_current_premium() -> TestResult {
    native_case(
        "m01b-berserker-entry",
        "m01b/berserker-treasure/s1/white_pearls",
        33198,
        31923,
        -11,
    )
}
