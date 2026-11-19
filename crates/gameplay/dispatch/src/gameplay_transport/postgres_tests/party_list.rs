//! T172: real admitted party dispatch projects only nearby vitals.
use crate::bestiary_postgres_harness::TestResult;


#[test]
fn party_list_real_dispatch_range_and_legacy_gate_postgres() -> TestResult {
    super::party_buffs_fixtures::run_party_list()
}
