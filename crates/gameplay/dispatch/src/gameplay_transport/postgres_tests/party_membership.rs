//! PARTY-1 real admitted transport, native durable writer and reloaded owner.
use super::party_membership_fixtures::{run};

use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_membership_real_dispatch_postgres_end_to_end() -> TestResult {
    run(false)
}
