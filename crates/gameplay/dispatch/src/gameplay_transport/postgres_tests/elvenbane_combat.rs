//! Source-native wire combat continuation; no fixture damage, health or kill flags.
use super::TestResult;


#[test]
fn elvenbane_source_native_dragon_earned_via_wire() -> TestResult {
    super::annihilator_route_fixtures::elvenbane_source_combat()
}

#[test]
fn elvenbane_source_all_six_defenders_earned_via_wire() -> TestResult {
    super::annihilator_route_fixtures::elvenbane_source_defenders()
}
