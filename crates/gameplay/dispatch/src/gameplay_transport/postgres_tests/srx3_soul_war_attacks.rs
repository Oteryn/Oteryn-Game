//! Real native proposals survive cancellation after a committed ordinary hit.
use super::TestResult;

#[test]
fn srx3_soul_war_native_attack_suffix_survives_physical_cancellation_postgres() -> TestResult {
    super::srx3_cruelty_callbacks_fixtures::run_source_entry(true)
}
