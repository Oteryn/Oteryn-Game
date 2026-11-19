//! MRL-3: current SQL party membership feeds the real scheduled monster reader.
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn reference_level_party_lookup_postgres_real_dispatch() -> TestResult {
    super::party_membership_fixtures::run_reference_level()
}
