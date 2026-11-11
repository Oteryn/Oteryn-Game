//! Exact source startup door keys, without fixture-authored action attributes.
use super::TestResult;

#[test]
fn questpc6_fibula_keys_3940_unlocks_source_entrance_and_retains_custody() -> TestResult {
    super::m06_keys::source_key_door(32190, 32432, -8, 3940)
}

#[test]
fn questpc6_fibula_keys_3980_unlocks_source_cavern_and_retains_custody() -> TestResult {
    super::m06_keys::source_key_door(32277, 32420, -10, 3980)
}
