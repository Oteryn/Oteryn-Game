//! Party notices reach both live members independently of the liveness cadence.
use crate::bestiary_postgres_harness::TestResult;

#[test]
fn party_notices_reach_every_member_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    super::party_membership_fixtures::run_fanout_notices(admin)
}
