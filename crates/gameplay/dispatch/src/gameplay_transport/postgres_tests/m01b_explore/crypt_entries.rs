use super::*;
use super::super::m01b_explore_fixtures::crypt_entries::*;

#[test]
fn m01b_crypt_inukaya_native_entry_requires_current_premium() -> TestResult {
    native_case(
        "m01b-inukaya-entry",
        "m01b/inukaya/s1/inukaya",
        32484,
        31109,
        -8,
    )
}

#[test]
fn m01b_crypt_medusa_native_entry_requires_level_and_current_premium() -> TestResult {
    native_case(
        "m01b-medusa-entry",
        "m01b/medusa-shield/s1/the_medusa_quest",
        33050,
        32398,
        -10,
    )
}
